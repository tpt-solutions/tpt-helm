// SPDX-License-Identifier: MIT OR Apache-2.0

//! Security-focused integration tests: simulated GPS spoofing scenarios.
//!
//! These tests exercise the detector end-to-end against adversarial GPS inputs:
//! a slowly drifting spoof (meant to evade naive threshold checks), a step
//! spoof (instant jump), a "perfect" spoof that fools only one reference, and
//! a coordinated spoof that fools both. They also confirm the system does NOT
//! cry wolf during legitimate operation (jamming-free, sensor-noise regime).

#![allow(clippy::expect_used, clippy::unnecessary_wraps)]

use std::any::Any;

use tp_helm_spoof::celestial::{CelestialCrossCheck, CelestialFix};
use tp_helm_spoof::detector::{Detector, DetectorConfig, Severity};
use tp_helm_spoof::geo::Position;
use tp_helm_spoof::inertial::{InertialCrossCheck, InertialFix};

/// Assemble the standard two-reference detector.
fn detector() -> Detector {
    Detector::new(
        vec![Box::new(InertialCrossCheck), Box::new(CelestialCrossCheck)],
        DetectorConfig::default(),
    )
}

/// Box a value as the `dyn Any` payload the detector consumes.
fn payload<T: Any + 'static>(v: T) -> Option<Box<dyn Any>> {
    Some(Box::new(v))
}

#[test]
fn legitimate_operation_does_not_alert() {
    // Truth at sea; GPS, INS, and celestial all agree within noise.
    let truth = Position::new(-122.0, 37.0);
    let gps = Position::new(-122.0002, 37.0002);
    let ins = InertialFix::new(Position::new(-122.0003, 37.0003), 30.0);
    // Celestial fix within its ~1 NM accuracy envelope of the true position.
    let cel = CelestialFix::new(Position::new(-122.002, 37.002));
    let alert = detector().evaluate(&gps, &[payload(ins), payload(cel)]);
    // Legitimate operation must never raise an actionable (Warning/Alarm) alert.
    assert!(
        !alert
            .as_ref()
            .is_some_and(tp_helm_spoof::detector::Alert::is_actionable),
        "truth={truth:?} must not raise an actionable alert: {alert:?}"
    );
}

#[test]
fn step_spoof_is_detected() {
    // GPS jumps 8 km north while INS/celestial hold truth.
    let truth = Position::new(-122.0, 37.0);
    let gps = Position::new(-122.0, 37.072); // ~8 km
    let ins = InertialFix::new(truth, 30.0);
    let cel = CelestialFix::new(truth);
    let alert = detector()
        .evaluate(&gps, &[payload(ins), payload(cel)])
        .expect("step spoof detected");
    assert_eq!(alert.severity, Severity::Alarm);
}

#[test]
fn slowly_drifting_spoof_eventually_detected() {
    // A meaconing attack that drifts the reported position 50 m per sample.
    // Each single step is small, but cumulative drift must trip the alarm
    // before it becomes operationally dangerous.
    let truth = Position::new(-122.0, 37.0);
    let mut gps_lat = 37.0;
    let mut detected_at_steps: Option<usize> = None;

    for step in 1..=200 {
        gps_lat += 0.00045; // ~50 m northward
        let gps = Position::new(-122.0, gps_lat);
        let ins = InertialFix::new(truth, 30.0);
        let cel = CelestialFix::new(truth);
        if let Some(alert) = detector().evaluate(&gps, &[payload(ins), payload(cel)]) {
            if alert.is_actionable() {
                detected_at_steps = Some(step);
                break;
            }
        }
    }

    let steps = detected_at_steps.expect("drifting spoof must be detected");
    // Detected while cumulative drift is still small: well under 1 NM.
    let drift_m = f64::from(u32::try_from(steps).unwrap_or(0)) * 50.0;
    assert!(drift_m < 1852.0, "should detect at {drift_m:.0} m < 1 NM");
}

#[test]
fn spoof_fooling_only_one_reference_still_detected() {
    // Attacker injects a false celestial fix matching the spoofed GPS, but the
    // inertial reference is untouched. The detector must not be fooled because
    // at least one independent reference disagrees.
    let truth = Position::new(-122.0, 37.0);
    let spoofed = Position::new(-122.0, 37.05);
    let gps = spoofed;
    let ins = InertialFix::new(truth, 30.0); // honest
    let cel = CelestialFix::new(spoofed); // compromised to match spoof
    let alert = detector()
        .evaluate(&gps, &[payload(ins), payload(cel)])
        .expect("single-reference compromise detected");
    assert!(alert.is_actionable());
}

#[test]
fn partially_coordinated_spoof_where_references_disagree_is_detected() {
    // The attacker spoofs GPS and the celestial reference to the same false
    // location, but the inertial reference is *not* compromised. Because the
    // references disagree with each other, the GPS-vs-INS cross-check still
    // flags the spoof. This is the realistic, detectable case.
    let truth = Position::new(-122.0, 37.0);
    let spoofed = Position::new(-122.0, 37.05);
    let gps = spoofed;
    let ins = InertialFix::new(truth, 30.0); // honest
    let cel = CelestialFix::new(spoofed); // compromised to match GPS
    let alert = detector()
        .evaluate(&gps, &[payload(ins), payload(cel)])
        .expect("spoof detected via INS/celestial disagreement");
    assert!(alert.is_actionable());
}

#[test]
fn fully_coordinated_spoof_with_matched_velocity_is_a_documented_limitation() {
    // If an attacker perfectly spoofs GPS AND both independent references to the
    // exact same location, and matches the vessel's true velocity profile, no
    // positional cross-check can detect the deception. This is a documented
    // limitation of position-only cross-checks; defeating it requires
    // authentication of the reference sources and/or velocity-consistency
    // monitoring (tracked in todo.md Phase 4 follow-ups). We assert the detector
    // is honest about this: it does NOT raise a false actionable alert.
    let spoofed = Position::new(-122.0, 37.0);
    let gps = spoofed;
    let ins = InertialFix::new(spoofed, 30.0);
    let cel = CelestialFix::new(spoofed);
    let alert = detector().evaluate(&gps, &[payload(ins), payload(cel)]);
    assert!(
        !alert
            .as_ref()
            .is_some_and(tp_helm_spoof::detector::Alert::is_actionable),
        "fully coordinated spoof must not produce a false actionable alert: {alert:?}"
    );
}

#[test]
fn missing_celestial_at_night_still_protected_by_ins() {
    let truth = Position::new(-122.0, 37.0);
    let gps = Position::new(-122.0, 37.06); // spoofed 6+ km
    let ins = InertialFix::new(truth, 30.0);
    // No celestial fix available (overcast/night).
    let alert = detector()
        .evaluate(&gps, &[payload(ins), None])
        .expect("INS-only detects spoof");
    assert!(alert.is_actionable());
}

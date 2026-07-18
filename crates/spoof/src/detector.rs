// SPDX-License-Identifier: MIT OR Apache-2.0

//! Spoofing detection / alerting engine.
//!
//! The detector fuses two (or more) independent, RF-independent navigation
//! references — inertial and celestial — against the GPS fix. Each
//! [`CrossCheck`] yields a residual distance and a reference uncertainty
//! envelope. A residual far larger than the combined uncertainty is evidence
//! that the GPS fix is not where it claims to be, i.e. a spoofing or
//! interference event.
//!
//! Decisions are made on a confidence score in `0.0..=1.0` derived from how
//! many standard deviations the residual exceeds the expected noise, combined
//! across all available references. When the score crosses a configurable
//! threshold the detector raises an [`Alert`].

use crate::geo::Position;

/// Which independent reference a cross-check represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceSource {
    /// Inertial navigation system / IMU dead-reckoning.
    Inertial,
    /// Celestial (sextant/chronometer) fix.
    Celestial,
}

/// Outcome of comparing the GPS fix against one reference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossCheckResult {
    /// Great-circle residual between GPS and the reference (meters).
    pub residual_m: f64,
    /// 1-sigma uncertainty of the reference (meters).
    pub reference_error_m: f64,
}

/// A cross-checkable independent navigation reference.
pub trait CrossCheck {
    /// The source this check represents.
    fn source(&self) -> ReferenceSource;

    /// Compare `gps` against the reference payload. Returns `None` if the
    /// reference payload is the wrong type for this check.
    fn compare(&self, gps: &Position, reference: &dyn std::any::Any) -> Option<CrossCheckResult>;
}

/// Severity of a spoofing alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Residual exceeds the reference error but is borderline; watch only.
    Watch,
    /// Strong evidence the GPS fix is inconsistent with independent references.
    Warning,
    /// High-confidence spoofing / interference detection.
    Alarm,
}

/// A raised spoofing alert with the contributing evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct Alert {
    /// Overall confidence score in `0.0..=1.0`.
    pub confidence: f64,
    /// Severity classification.
    pub severity: Severity,
    /// Per-reference residuals that contributed to the decision.
    pub evidence: Vec<(ReferenceSource, CrossCheckResult)>,
}

impl Alert {
    /// True when the alert is actionable (warning or alarm).
    #[must_use]
    pub fn is_actionable(&self) -> bool {
        matches!(self.severity, Severity::Warning | Severity::Alarm)
    }
}

/// Detector configuration: thresholds on the confidence score.
#[derive(Debug, Clone, Copy)]
pub struct DetectorConfig {
    /// Confidence at or above which a Warning is raised.
    pub warning_threshold: f64,
    /// Confidence at or above which an Alarm is raised.
    pub alarm_threshold: f64,
    /// Residual (in units of combined sigma) at which confidence saturates to 1.
    pub saturation_sigma: f64,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 0.75,
            alarm_threshold: 0.92,
            saturation_sigma: 6.0,
        }
    }
}

/// GPS spoofing / interference detector.
pub struct Detector {
    checks: Vec<Box<dyn CrossCheck>>,
    config: DetectorConfig,
}

impl std::fmt::Debug for Detector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Detector")
            .field("config", &self.config)
            .field("checks", &self.checks.len())
            .finish()
    }
}

impl Detector {
    /// Build a detector over the given cross-checks.
    #[must_use]
    pub fn new(checks: Vec<Box<dyn CrossCheck>>, config: DetectorConfig) -> Self {
        Self { checks, config }
    }

    /// Run every cross-check and produce an [`Alert`] if warranted.
    ///
    /// `references` pairs each check's expected payload with the boxed value the
    /// caller supplies; a `None` entry means that reference is unavailable for
    /// this cycle (e.g. no celestial fix at night) and is skipped. The detector
    /// still raises an alert if any *available* reference disagrees strongly.
    #[must_use]
    pub fn evaluate(
        &self,
        gps: &Position,
        references: &[Option<Box<dyn std::any::Any>>],
    ) -> Option<Alert> {
        let mut evidence = Vec::new();
        for (check, reference) in self.checks.iter().zip(references.iter()) {
            let Some(payload) = reference else {
                continue;
            };
            if let Some(result) = check.compare(gps, payload.as_ref()) {
                evidence.push((check.source(), result));
            }
        }

        if evidence.is_empty() {
            return None;
        }

        // Take the worst (largest) confidence across references: a single
        // trustworthy independent system flagging a gross mismatch is enough.
        let mut worst_confidence = 0.0_f64;
        for (_src, result) in &evidence {
            let sigma = result.reference_error_m.max(1.0);
            let n_sigma = result.residual_m / sigma;
            let confidence = confidence_from_sigma(n_sigma, self.config.saturation_sigma);
            worst_confidence = worst_confidence.max(confidence);
        }

        let severity = if worst_confidence >= self.config.alarm_threshold {
            Severity::Alarm
        } else if worst_confidence >= self.config.warning_threshold {
            Severity::Warning
        } else {
            Severity::Watch
        };

        Some(Alert {
            confidence: worst_confidence,
            severity,
            evidence,
        })
    }
}

/// Map a residual expressed in sigma units to a `0..=1` confidence via a
/// smooth saturating curve (logistic), capped at the saturation point.
fn confidence_from_sigma(n_sigma: f64, saturation_sigma: f64) -> f64 {
    if n_sigma <= 0.0 {
        return 0.0;
    }
    let x = (n_sigma / saturation_sigma) * 6.0;
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::celestial::{CelestialCrossCheck, CelestialFix};
    use crate::inertial::{InertialCrossCheck, InertialFix};

    fn detector() -> Detector {
        Detector::new(
            vec![Box::new(InertialCrossCheck), Box::new(CelestialCrossCheck)],
            DetectorConfig::default(),
        )
    }

    #[test]
    fn consistent_fix_is_no_alert() {
        let gps = Position::new(-122.0, 37.0);
        let ins = InertialFix::new(Position::new(-122.0001, 37.0001), 10.0);
        let cel = CelestialFix::new(Position::new(-122.005, 37.005));
        let alert = detector().evaluate(&gps, &[Some(Box::new(ins)), Some(Box::new(cel))]);
        // Consistent references yield at most a non-actionable Watch; they must
        // never raise an actionable (Warning/Alarm) false alarm.
        assert!(
            !alert.as_ref().is_some_and(Alert::is_actionable),
            "consistent references must not raise an actionable alert: {alert:?}"
        );
    }

    #[test]
    fn gross_mismatch_raises_alarm() {
        let gps = Position::new(-122.0, 37.0);
        // GPS claims a position 10 km from the INS truth.
        let ins = InertialFix::new(Position::new(-122.0, 37.09), 10.0);
        let cel = CelestialFix::new(Position::new(-122.0, 37.09));
        let alert = detector()
            .evaluate(&gps, &[Some(Box::new(ins)), Some(Box::new(cel))])
            .expect("alert raised");
        assert_eq!(alert.severity, Severity::Alarm);
        assert!(alert.is_actionable());
        assert!(alert.confidence >= 0.92);
    }

    #[test]
    fn missing_reference_is_skipped_not_false_alarm() {
        let gps = Position::new(-122.0, 37.0);
        let ins = InertialFix::new(Position::new(-122.0001, 37.0001), 10.0);
        // No celestial fix available; must not panic and must not raise an
        // actionable alert based solely on the (consistent) INS reference.
        let alert = detector().evaluate(&gps, &[Some(Box::new(ins)), None]);
        assert!(
            !alert.as_ref().is_some_and(Alert::is_actionable),
            "missing celestial must not cause a false actionable alert: {alert:?}"
        );
    }

    #[test]
    fn confidence_is_monotonic_in_residual() {
        let a = confidence_from_sigma(1.0, 6.0);
        let b = confidence_from_sigma(3.0, 6.0);
        let c = confidence_from_sigma(10.0, 6.0);
        assert!(a < b);
        assert!(b < c);
        assert!(
            (c - 1.0).abs() < 1e-3,
            "confidence should saturate to 1: {c}"
        );
    }
}

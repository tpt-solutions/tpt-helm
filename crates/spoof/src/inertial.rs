// SPDX-License-Identifier: MIT OR Apache-2.0

//! Inertial navigation cross-check.
//!
//! An Inertial Navigation System (INS) / IMU dead-reckons the vessel's
//! position from body-frame accelerations and rotation rates, independent of
//! any RF signal. Over short horizons the INS position is a trustworthy
//! reference that does not depend on GPS, so a discrepancy between the GPS fix
//! and the INS-derived position is a strong spoofing / interference indicator.
//!
//! The INS output here is modeled as a position plus a drift error that grows
//! with time since the last alignment. The cross-check compares the two
//! positions and returns a residual distance together with an uncertainty
//! envelope so the detector can distinguish a genuine disagreement from
//! ordinary sensor noise.

use crate::detector::{CrossCheck, CrossCheckResult, ReferenceSource};
use crate::geo::Position;

/// Inertial-derived position fix with an associated circular error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InertialFix {
    /// INS-estimated position (WGS84, degrees).
    pub position: Position,
    /// Seconds elapsed since the last INS alignment / zero-velocity update.
    pub time_since_align_s: f64,
    /// 1-sigma horizontal position error (meters) at alignment.
    pub base_error_m: f64,
    /// Error growth rate (meters per second of unaligned drift).
    pub drift_rate_mps: f64,
}

impl InertialFix {
    /// Build an inertial fix from a position and alignment age.
    #[must_use]
    pub fn new(position: Position, time_since_align_s: f64) -> Self {
        Self {
            position,
            time_since_align_s,
            // A marine-grade INS typically holds ~0.1 NM (≈185 m) horizontal
            // error shortly after alignment; this is the conservative floor.
            base_error_m: 185.0,
            // Schuler-aligned INS drift is slow; 0.05 m/s is a conservative
            // upper bound for uncompensated horizontal error growth.
            drift_rate_mps: 0.05,
        }
    }

    /// Current 1-sigma horizontal error including drift since alignment (meters).
    #[must_use]
    pub fn error_m(&self) -> f64 {
        self.base_error_m + self.drift_rate_mps * self.time_since_align_s.max(0.0)
    }
}

/// Cross-check the GPS fix against an inertial reference.
///
/// Returns the great-circle residual between the two positions and the INS
/// uncertainty envelope. A small residual well inside the envelope is normal;
/// a residual many times larger than the combined error is suspicious.
pub struct InertialCrossCheck;

impl CrossCheck for InertialCrossCheck {
    fn source(&self) -> ReferenceSource {
        ReferenceSource::Inertial
    }

    fn compare(&self, gps: &Position, reference: &dyn std::any::Any) -> Option<CrossCheckResult> {
        let fix = reference.downcast_ref::<InertialFix>()?;
        let residual_m = gps.distance_m(&fix.position);
        let reference_error_m = fix.error_m();
        Some(CrossCheckResult {
            residual_m,
            reference_error_m,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn matching_fix_yields_small_residual() {
        let gps = Position::new(-122.0, 37.0);
        let ins = InertialFix::new(Position::new(-122.0001, 37.0001), 10.0);
        let r = InertialCrossCheck
            .compare(&gps, &ins)
            .expect("downcast succeeds");
        assert!(
            r.residual_m < r.reference_error_m,
            "residual {:.1} m must be < error {:.1} m",
            r.residual_m,
            r.reference_error_m
        );
    }

    #[test]
    fn large_offset_exceeds_envelope() {
        let gps = Position::new(-122.0, 37.0);
        let ins = InertialFix::new(Position::new(-122.1, 37.1), 10.0);
        let r = InertialCrossCheck
            .compare(&gps, &ins)
            .expect("downcast succeeds");
        assert!(
            r.residual_m > 5.0 * r.reference_error_m,
            "spoof offset should dominate INS error"
        );
    }

    #[test]
    fn drift_grows_with_alignment_age() {
        let fresh = InertialFix::new(Position::new(0.0, 0.0), 0.0);
        let stale = InertialFix::new(Position::new(0.0, 0.0), 3600.0);
        assert!(stale.error_m() > fresh.error_m());
    }
}

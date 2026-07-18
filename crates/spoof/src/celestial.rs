// SPDX-License-Identifier: MIT OR Apache-2.0

//! Celestial navigation cross-check.
//!
//! Celestial navigation fixes the vessel's position from observed altitudes of
//! celestial bodies (sun, stars, planets) measured with a sextant and a
//! chronometer. It depends only on the vessel's clock and a clear sky, never
//! on RF signals, making it an independent, spoofing-resistant reference.
//!
//! The astronomic position is supplied as a fix with a deliberately conservative
//! circular error (typical noon-sight / star-sight accuracy is on the order of
//! a few nautical miles). The cross-check compares the GPS position to the
//! celestial fix and reports the residual scaled against the celestial error.

use crate::detector::{CrossCheck, CrossCheckResult, ReferenceSource};
use crate::geo::Position;

/// Celestial-derived position fix with its circular error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CelestialFix {
    /// Celestial-estimated position (WGS84, degrees).
    pub position: Position,
    /// 1-sigma horizontal error of the celestial fix (meters).
    pub error_m: f64,
}

impl CelestialFix {
    /// Build a celestial fix; defaults to a conservative 1 NM error.
    #[must_use]
    pub fn new(position: Position) -> Self {
        Self {
            position,
            error_m: 1852.0,
        }
    }
}

/// Cross-check the GPS fix against a celestial reference.
pub struct CelestialCrossCheck;

impl CrossCheck for CelestialCrossCheck {
    fn source(&self) -> ReferenceSource {
        ReferenceSource::Celestial
    }

    fn compare(&self, gps: &Position, reference: &dyn std::any::Any) -> Option<CrossCheckResult> {
        let fix = reference.downcast_ref::<CelestialFix>()?;
        let residual_m = gps.distance_m(&fix.position);
        Some(CrossCheckResult {
            residual_m,
            reference_error_m: fix.error_m,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn residual_within_celestial_error_is_normal() {
        let gps = Position::new(-122.0, 37.0);
        let cel = CelestialFix::new(Position::new(-122.005, 37.005));
        let r = CelestialCrossCheck
            .compare(&gps, &cel)
            .expect("downcast succeeds");
        assert!(
            r.residual_m < r.reference_error_m,
            "residual {:.1} m < error {:.1} m",
            r.residual_m,
            r.reference_error_m
        );
    }

    #[test]
    fn gross_spoof_exceeds_celestial_error() {
        let gps = Position::new(-122.0, 37.0);
        let cel = CelestialFix::new(Position::new(-121.0, 37.0));
        let r = CelestialCrossCheck
            .compare(&gps, &cel)
            .expect("downcast succeeds");
        assert!(r.residual_m > 3.0 * r.reference_error_m);
    }
}

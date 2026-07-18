// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared geospatial helpers for spoofing cross-checks.
//!
//! Provides a lightweight [`Position`] type and great-circle distance in meters,
//! sufficient for comparing fixes from independent navigation references.

use serde::{Deserialize, Serialize};

const EARTH_RADIUS_M: f64 = 6_371_000.0;
const DEG: f64 = std::f64::consts::PI / 180.0;

/// A geographic position in degrees (WGS84).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// Longitude in degrees, -180..180.
    pub lon: f64,
    /// Latitude in degrees, -90..90.
    pub lat: f64,
}

impl Position {
    /// Construct a position.
    #[must_use]
    pub fn new(lon: f64, lat: f64) -> Self {
        Self { lon, lat }
    }

    /// Great-circle distance to `other` in meters.
    #[must_use]
    pub fn distance_m(&self, other: &Position) -> f64 {
        let lat1 = self.lat * DEG;
        let lat2 = other.lat * DEG;
        let dlat = (other.lat - self.lat) * DEG;
        let dlon = (other.lon - self.lon) * DEG;
        let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        2.0 * EARTH_RADIUS_M * h.sqrt().asin()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn one_degree_latitude_is_about_111_km() {
        let a = Position::new(0.0, 0.0);
        let b = Position::new(0.0, 1.0);
        let d = a.distance_m(&b);
        assert!((d - 111_000.0).abs() < 2_000.0, "unexpected {d:.0} m");
    }

    #[test]
    fn identical_points_have_zero_distance() {
        let a = Position::new(-122.0, 37.0);
        assert_eq!(a.distance_m(&a), 0.0);
    }
}

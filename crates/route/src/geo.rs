// SPDX-License-Identifier: MIT OR Apache-2.0

//! Geospatial helpers: positions, great-circle distance, and bearing.
//!
//! Routes are planned in WGS84 geographic coordinates. Short ocean legs are
//! well approximated by the spherical Earth (haversine) formulas, which keep
//! the planner fast and dependency-light.

use geo::Coord;
use serde::{Deserialize, Serialize};

const EARTH_RADIUS_M: f64 = 6_371_000.0;
const NM_PER_M: f64 = 1.0 / 1852.0;
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

    /// Convert to a `geo` coordinate.
    #[must_use]
    pub fn coord(&self) -> Coord<f64> {
        Coord {
            x: self.lon,
            y: self.lat,
        }
    }

    /// Great-circle distance to `other` in nautical miles.
    #[must_use]
    pub fn distance_nm(&self, other: &Position) -> f64 {
        Haversine::distance_nm(self.coord(), other.coord())
    }

    /// Initial bearing (degrees true, 0 = north, clockwise) toward `other`.
    #[must_use]
    pub fn bearing_to(&self, other: &Position) -> f64 {
        Haversine::bearing(self.coord(), other.coord())
    }
}

/// Great-circle geometry convenience wrappers.
pub struct Haversine;

impl Haversine {
    /// Distance between two coordinates in nautical miles.
    #[must_use]
    pub fn distance_nm(a: Coord<f64>, b: Coord<f64>) -> f64 {
        let lat1 = a.y * DEG;
        let lat2 = b.y * DEG;
        let dlat = (b.y - a.y) * DEG;
        let dlon = (b.x - a.x) * DEG;
        let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * h.sqrt().asin();
        EARTH_RADIUS_M * c * NM_PER_M
    }

    /// Initial bearing (degrees true, 0..360) from `a` to `b`.
    #[must_use]
    pub fn bearing(a: Coord<f64>, b: Coord<f64>) -> f64 {
        let lat1 = a.y * DEG;
        let lat2 = b.y * DEG;
        let dlon = (b.x - a.x) * DEG;
        let y = dlon.sin() * lat2.cos();
        let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
        let bearing = y.atan2(x) / DEG;
        (bearing + 360.0) % 360.0
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn distance_sf_to_la_approximate() {
        // San Francisco to Los Angeles is ~ 340 nm by sea.
        let sf = Position::new(-122.42, 37.77);
        let la = Position::new(-118.24, 33.74);
        let d = sf.distance_nm(&la);
        assert!((d - 340.0).abs() < 30.0, "unexpected distance {d}");
    }

    #[test]
    fn bearing_north_is_zero() {
        let a = Position::new(0.0, 0.0);
        let b = Position::new(0.0, 1.0);
        assert!((a.bearing_to(&b)).abs() < 1e-6);
    }

    #[test]
    fn bearing_east_is_ninety() {
        let a = Position::new(0.0, 0.0);
        let b = Position::new(1.0, 0.0);
        assert!((a.bearing_to(&b) - 90.0).abs() < 1e-6);
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0

//! Weather and ocean-current data interfaces for route optimization.
//!
//! The planner consumes environmental fields sampled along the route. These
//! traits define the ingestion boundary: concrete providers (GRIB files, a
//! network forecast service, or test fixtures) implement [`WeatherField`] and
//! [`Current`] without the planner caring about the source.

use crate::geo::Position;
use serde::{Deserialize, Serialize};

/// Wind state at a point: direction the wind is *coming from* (meteorological
/// convention, degrees true) and speed in knots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Wind {
    /// Direction wind blows *from*, degrees true (0 = from north).
    pub from_bearing: f64,
    /// Wind speed in knots.
    pub speed_kn: f64,
}

/// Ocean surface current at a point: set (direction *toward*, degrees true)
/// and drift speed in knots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Current {
    /// Direction the current flows *toward*, degrees true.
    pub set_bearing: f64,
    /// Drift speed in knots.
    pub speed_kn: f64,
}

/// A source of wind and wave parameters along a route.
///
/// Implementors return the environmental state at an arbitrary position. The
/// planner samples this at each candidate leg midpoint.
pub trait WeatherField {
    /// Wind at `pos`, if the field covers it.
    fn wind_at(&self, pos: &Position) -> Option<Wind>;

    /// Significant wave height (meters) at `pos`, if known.
    fn wave_height_m(&self, pos: &Position) -> Option<f64>;
}

/// A source of ocean currents along a route.
pub trait CurrentSource {
    /// Surface current at `pos`, if the source covers it.
    fn current_at(&self, pos: &Position) -> Option<Current>;
}

/// A trivially calm environment: no wind, no current, no waves.
///
/// Useful as a baseline and in tests where the optimum should reduce to the
/// great-circle (straight) path.
#[derive(Debug, Clone, Copy, Default)]
pub struct CalmEnvironment;

impl WeatherField for CalmEnvironment {
    fn wind_at(&self, _pos: &Position) -> Option<Wind> {
        Some(Wind {
            from_bearing: 0.0,
            speed_kn: 0.0,
        })
    }

    fn wave_height_m(&self, _pos: &Position) -> Option<f64> {
        Some(0.0)
    }
}

impl CurrentSource for CalmEnvironment {
    fn current_at(&self, _pos: &Position) -> Option<Current> {
        Some(Current {
            set_bearing: 0.0,
            speed_kn: 0.0,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn calm_environment_reports_zero() {
        let env = CalmEnvironment;
        let p = Position::new(0.0, 0.0);
        assert_eq!(env.wind_at(&p).expect("wind").speed_kn, 0.0);
        assert_eq!(env.current_at(&p).expect("current").speed_kn, 0.0);
        assert_eq!(env.wave_height_m(&p).expect("wave"), 0.0);
    }
}

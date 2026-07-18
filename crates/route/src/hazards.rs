// SPDX-License-Identifier: MIT OR Apache-2.0

//! Navigation hazards and constraints applied as hard obstacles.
//!
//! The planner must never route through these. They model:
//! * [`RestrictedArea`] — permanent no-go polygons (land, military zones).
//! * [`TrafficZone`] — conditionally active separation/canal zones.
//!
//! A [`Hazard`] enum unifies them for storage and intersection testing.

use crate::geo::Position;
use geo::{Contains, Coord, LineString, Point, Polygon};

/// A permanently closed area the vessel may not enter (e.g. land, shoals).
#[derive(Debug, Clone, PartialEq)]
pub struct RestrictedArea {
    /// Human-readable name / charted feature.
    pub name: String,
    /// Closed polygon boundary (WGS84 degrees).
    pub polygon: Polygon<f64>,
}

/// A traffic management zone (e.g. traffic separation scheme, canal).
///
/// Unlike [`RestrictedArea`], a traffic zone may be active only during a
/// time window; when inactive it does not constrain the route.
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficZone {
    pub name: String,
    pub polygon: Polygon<f64>,
    /// If set, the zone is avoided only when the route's planned transit
    /// `hour` falls within `[active_from, active_to)`.
    pub active_from: Option<u32>,
    pub active_to: Option<u32>,
}

/// A unified navigation hazard.
#[derive(Debug, Clone, PartialEq)]
pub enum Hazard {
    Restricted(RestrictedArea),
    Traffic(TrafficZone),
}

impl Hazard {
    /// True if `pos` lies inside this hazard.
    #[must_use]
    pub fn contains(&self, pos: &Position) -> bool {
        let p: Point<f64> = Point::new(pos.lon, pos.lat);
        match self {
            Hazard::Restricted(r) => r.polygon.contains(&p),
            Hazard::Traffic(t) => t.polygon.contains(&p),
        }
    }

    /// Does the straight segment `a`→`b` enter this hazard?
    ///
    /// We test the endpoints and a set of intermediate samples. This is an
    /// approximation sufficient for planning (the leg length is bounded by the
    /// planner's candidate step size); exact segment/polygon intersection is
    /// not required for a hard obstacle check.
    #[must_use]
    pub fn intersects_segment(&self, a: &Position, b: &Position) -> bool {
        if self.contains(a) || self.contains(b) {
            return true;
        }
        let samples = 16;
        for i in 1..samples {
            let t = f64::from(i) / f64::from(samples);
            let lon = a.lon + (b.lon - a.lon) * t;
            let lat = a.lat + (b.lat - a.lat) * t;
            if self.contains(&Position::new(lon, lat)) {
                return true;
            }
        }
        false
    }

    /// Whether this hazard is active at planning `hour` (for traffic zones).
    #[must_use]
    pub fn active_at(&self, hour: u32) -> bool {
        match self {
            Hazard::Restricted(_) => true,
            Hazard::Traffic(t) => match (t.active_from, t.active_to) {
                (None, None) => true,
                (Some(f), Some(to)) => hour >= f && hour < to,
                (Some(f), None) => hour >= f,
                (None, Some(to)) => hour < to,
            },
        }
    }
}

/// Build a rectangular [`RestrictedArea`] from two corner positions.
#[must_use]
pub fn rectangular_restriction(name: &str, sw: Position, ne: Position) -> RestrictedArea {
    let nw = Coord {
        x: sw.lon,
        y: ne.lat,
    };
    let se = Coord {
        x: ne.lon,
        y: sw.lat,
    };
    let ring = LineString(vec![sw.coord(), se, ne.coord(), nw, sw.coord()]);
    RestrictedArea {
        name: name.to_string(),
        polygon: Polygon::new(ring, vec![]),
    }
}

/// A helper to test whether a straight leg crosses any active hazard.
#[must_use]
pub fn leg_blocked(a: &Position, b: &Position, hazards: &[Hazard], hour: u32) -> bool {
    hazards
        .iter()
        .filter(|h| h.active_at(hour))
        .any(|h| h.intersects_segment(a, b))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use geo::line_string;

    fn box_hazard() -> RestrictedArea {
        rectangular_restriction("land", Position::new(0.0, 0.0), Position::new(1.0, 1.0))
    }

    #[test]
    fn point_inside_detected() {
        let h = Hazard::Restricted(box_hazard());
        assert!(h.contains(&Position::new(0.5, 0.5)));
        assert!(!h.contains(&Position::new(2.0, 2.0)));
    }

    #[test]
    fn segment_crossing_detected() {
        let h = Hazard::Restricted(box_hazard());
        let a = Position::new(-1.0, 0.5);
        let b = Position::new(2.0, 0.5);
        assert!(h.intersects_segment(&a, &b));
    }

    #[test]
    fn segment_clear_is_allowed() {
        let h = Hazard::Restricted(box_hazard());
        let a = Position::new(-1.0, -1.0);
        let b = Position::new(-1.0, 2.0);
        assert!(!h.intersects_segment(&a, &b));
    }

    #[test]
    fn traffic_zone_active_window() {
        let poly = Polygon::new(
            line_string![(x: 0.0, y: 0.0), (x: 1.0, y: 0.0), (x: 1.0, y: 1.0), (x: 0.0, y: 1.0), (x: 0.0, y: 0.0)],
            vec![],
        );
        let tz = Hazard::Traffic(TrafficZone {
            name: "ts".into(),
            polygon: poly,
            active_from: Some(8),
            active_to: Some(16),
        });
        assert!(tz.active_at(10));
        assert!(!tz.active_at(20));
    }

    #[test]
    fn leg_blocked_flags_hazards() {
        let h = Hazard::Restricted(box_hazard());
        let hazards = [h];
        assert!(leg_blocked(
            &Position::new(-1.0, 0.5),
            &Position::new(2.0, 0.5),
            &hazards,
            0
        ));
        assert!(!leg_blocked(
            &Position::new(-1.0, -1.0),
            &Position::new(-1.0, 2.0),
            &hazards,
            0
        ));
    }
}

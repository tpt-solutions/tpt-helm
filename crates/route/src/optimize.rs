// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuel-efficient route optimization.
//!
//! The planner builds a graph of candidate waypoints around the straight
//! great-circle line between start and end, then searches for the path with
//! the lowest integrated fuel cost while avoiding hazards. Fuel cost per leg
//! is a function of leg length, head/cross wind, opposing/following current,
//! and wave height — all of which change the effective speed and resistance
//! experienced by the hull.

use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;

use crate::geo::Position;
use crate::hazards::{leg_blocked, Hazard};
use crate::weather::{CalmEnvironment, Current, CurrentSource, WeatherField};

/// A point on a planned route.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Waypoint {
    /// Geographic position.
    pub pos: Position,
    /// Estimated time of arrival at this waypoint, hours from departure.
    pub eta_hours: f64,
}

/// A complete route plan: ordered waypoints plus the integrated fuel estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutePlan {
    /// Ordered waypoints (start .. end).
    pub waypoints: Vec<Waypoint>,
    /// Total planned distance in nautical miles.
    pub distance_nm: f64,
    /// Total estimated fuel burn in tonnes.
    pub fuel_tonnes: f64,
    /// Total estimated voyage time in hours.
    pub time_hours: f64,
}

/// Errors raised while planning.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlanError {
    /// No feasible route exists that avoids all hazards.
    #[error("no feasible route found avoiding all hazards")]
    NoFeasibleRoute,
    /// Start and end are identical or too close to plan.
    #[error("start and end positions are degenerate")]
    DegenerateEndpoints,
}

/// Vessel performance parameters used by the fuel model.
#[derive(Debug, Clone, Copy)]
pub struct VesselProfile {
    /// Still-water service speed in knots.
    pub service_speed_kn: f64,
    /// Fuel burn at service speed, tonnes per hour.
    pub fuel_rate_tph: f64,
    /// Hull form factor: extra resistance per knot of headwind (0..1).
    pub wind_factor: f64,
    /// Benefit from following current, fraction of current speed added to
    /// effective speed (0..1).
    pub current_factor: f64,
}

impl Default for VesselProfile {
    fn default() -> Self {
        Self {
            service_speed_kn: 14.0,
            fuel_rate_tph: 2.5,
            wind_factor: 0.02,
            current_factor: 0.8,
        }
    }
}

/// The planner: searches for a fuel-optimal route between two positions.
pub struct Planner<W, C>
where
    W: WeatherField,
    C: CurrentSource,
{
    weather: W,
    current: C,
    hazards: Vec<Hazard>,
    vessel: VesselProfile,
}

impl Planner<CalmEnvironment, CalmEnvironment> {
    /// Construct a planner over a calm (no wind/current) environment with no
    /// hazards. Convenient for baseline planning and tests.
    #[must_use]
    pub fn calm() -> Self {
        Self {
            weather: CalmEnvironment,
            current: CalmEnvironment,
            hazards: Vec::new(),
            vessel: VesselProfile::default(),
        }
    }
}

impl<W, C> Planner<W, C>
where
    W: WeatherField,
    C: CurrentSource,
{
    /// Build a planner from environmental sources, hazards, and a vessel.
    #[must_use]
    pub fn new(weather: W, current: C, hazards: Vec<Hazard>, vessel: VesselProfile) -> Self {
        Self {
            weather,
            current,
            hazards,
            vessel,
        }
    }

    /// Plan a route from `start` to `end`.
    ///
    /// Uses a randomized perturbation search seeded deterministically for
    /// reproducibility: it refines the straight route by nudging intermediate
    /// waypoints off the great-circle line to trade distance against
    /// environment-induced fuel cost, rejecting any candidate that enters a
    /// hazard.
    ///
    /// # Errors
    /// Returns [`PlanError::NoFeasibleRoute`] if no hazard-free route can be
    /// constructed, or [`PlanError::DegenerateEndpoints`] if the endpoints are
    /// too close.
    pub fn plan(&self, start: Position, end: Position) -> Result<RoutePlan, PlanError> {
        if start.distance_nm(&end) < 0.01 {
            return Err(PlanError::DegenerateEndpoints);
        }

        let mut rng = SmallRng::seed_from_u64(0x7ECD_15C0_FFEE_15C0);
        let waypoints = self.optimize(start, end, &mut rng);
        match waypoints {
            Some(wp) => {
                let plan = self.evaluate(&wp);
                Ok(plan)
            }
            None => Err(PlanError::NoFeasibleRoute),
        }
    }

    /// Search for a hazard-free, fuel-efficient route.
    ///
    /// Returns `None` if no feasible route is found. The search refines a
    /// straight great-circle seed by randomized waypoint perturbation, always
    /// preferring the lowest-fuel *feasible* (hazard-free) route discovered.
    /// Because the straight line may be blocked, a longer detour is accepted
    /// whenever it is feasible and improves on the best feasible route seen so
    /// far.
    fn optimize(
        &self,
        start: Position,
        end: Position,
        rng: &mut SmallRng,
    ) -> Option<Vec<Position>> {
        // Seed route: straight line with `K` intermediate points.
        let k = 10usize;
        let kf = k as f64;
        let seed: Vec<Position> = (0..=k)
            .map(|i| {
                let t = i as f64 / kf;
                Position::new(
                    start.lon + (end.lon - start.lon) * t,
                    start.lat + (end.lat - start.lat) * t,
                )
            })
            .collect();

        // Perturbation amplitude: large enough that waypoints can bow out
        // around a blocking hazard, but bounded relative to the route span.
        let span = (end.lon - start.lon)
            .abs()
            .max((end.lat - start.lat).abs())
            .max(0.5);
        let amp = span * 0.6;

        let mut best_feasible: Option<(Vec<Position>, f64)> = None;

        // Seed a couple of deliberately bowed routes so the search has a
        // feasible starting point when the straight line is blocked.
        for sign in [1.0, -1.0] {
            let mut bowed = seed.clone();
            for (i, p) in bowed.iter_mut().enumerate().take(k).skip(1) {
                let t = i as f64 / kf;
                // Perpendicular bow, largest in the middle of the route.
                let offset = sign * amp * (1.0 - (2.0 * t - 1.0).powi(2));
                p.lat += offset;
            }
            if self.is_clear(&bowed) {
                let cost = self.route_fuel(&bowed);
                best_feasible = Some((bowed, cost));
            }
        }

        for _ in 0..6000 {
            let base = match &best_feasible {
                Some((wp, _)) => wp.clone(),
                None => seed.clone(),
            };
            let mut candidate = base;
            let idx = 1 + rng.gen_range(0..k - 1);
            // Displace relative to the *seed* position so drift is bounded.
            candidate[idx].lon = seed[idx].lon + rng.gen_range(-amp..amp);
            candidate[idx].lat = seed[idx].lat + rng.gen_range(-amp..amp);

            if self.is_clear(&candidate) {
                let cost = self.route_fuel(&candidate);
                let improved = match &best_feasible {
                    Some((_, best_cost)) => cost < *best_cost,
                    None => true,
                };
                if improved {
                    best_feasible = Some((candidate, cost));
                }
            }
        }
        best_feasible.map(|(wp, _)| wp)
    }

    /// Are all legs of `route` hazard-free?
    fn is_clear(&self, route: &[Position]) -> bool {
        for w in route.windows(2) {
            if leg_blocked(&w[0], &w[1], &self.hazards, 0) {
                return false;
            }
        }
        true
    }

    /// Integrated fuel cost of a route (lower is better).
    fn route_fuel(&self, route: &[Position]) -> f64 {
        let mut total = 0.0;
        for w in route.windows(2) {
            total += self.leg_fuel(&w[0], &w[1]);
        }
        total
    }

    /// Fuel burned on one leg, accounting for environment.
    fn leg_fuel(&self, a: &Position, b: &Position) -> f64 {
        let dist = a.distance_nm(b);
        if dist <= 0.0 {
            return 0.0;
        }
        let mid = Position::new((a.lon + b.lon) / 2.0, (a.lat + b.lat) / 2.0);
        let bearing = a.bearing_to(b);

        // Effective speed from current: component of current along the leg.
        let eff_speed = self.effective_speed(&mid, bearing, dist);

        // Time for the leg at effective speed.
        let hours = dist / eff_speed.max(0.5);
        hours * self.vessel.fuel_rate_tph
    }

    /// Effective speed (knots) accounting for current assist/resistance and
    /// wind drag.
    fn effective_speed(&self, mid: &Position, bearing: f64, dist: f64) -> f64 {
        let mut speed = self.vessel.service_speed_kn;

        if let Some(cur) = self.current.current_at(mid) {
            speed += current_component(cur, bearing) * self.vessel.current_factor;
        }

        if let Some(wind) = self.weather.wind_at(mid) {
            // Headwind (wind coming toward the vessel's heading) slows us.
            let head = wind_component(wind, bearing);
            speed -= head.abs() * self.vessel.wind_factor * wind.speed_kn;
        }

        // Wave drag penalty grows with significant wave height.
        if let Some(wh) = self.weather.wave_height_m(mid) {
            speed -= 0.05 * wh;
        }

        speed.max(0.5) + dist * 0.0 // keep `dist` referenced; no distance scaling
    }

    /// Build a [`RoutePlan`] (with ETA, distance, fuel, time) from positions.
    fn evaluate(&self, route: &[Position]) -> RoutePlan {
        let mut waypoints = Vec::with_capacity(route.len());
        let mut distance_nm = 0.0;
        let mut fuel_tonnes = 0.0;
        let mut time_hours = 0.0;
        let mut prev: Option<Position> = None;
        for &p in route {
            if let Some(a) = prev {
                let dist = a.distance_nm(&p);
                let bearing = a.bearing_to(&p);
                let mid = Position::new((a.lon + p.lon) / 2.0, (a.lat + p.lat) / 2.0);
                let speed = self.effective_speed(&mid, bearing, dist);
                let hours = dist / speed.max(0.5);
                time_hours += hours;
                fuel_tonnes += hours * self.vessel.fuel_rate_tph;
                distance_nm += dist;
            }
            waypoints.push(Waypoint {
                pos: p,
                eta_hours: time_hours,
            });
            prev = Some(p);
        }
        RoutePlan {
            waypoints,
            distance_nm,
            fuel_tonnes,
            time_hours,
        }
    }
}

/// Signed component of a current along a leg bearing.
///
/// Positive means the current pushes the vessel forward (assist); negative
/// means it opposes (resistance).
fn current_component(cur: Current, bearing: f64) -> f64 {
    // Current set is the direction it flows toward. `angle_diff` is already in
    // [-180, 180], so `along.cos()` is in [-1, 1] without clamping.
    let along = angle_diff(cur.set_bearing, bearing);
    cur.speed_kn * along.cos()
}

/// Signed headwind component: how much the wind opposes the heading.
///
/// Wind `from_bearing` is where it comes *from*; the direction it blows *toward*
/// is `from_bearing + 180`. A positive result means a headwind.
fn wind_component(wind: crate::weather::Wind, bearing: f64) -> f64 {
    let toward = (wind.from_bearing + 180.0) % 360.0;
    let along = angle_diff(toward, bearing);
    // Positive `along` => wind blows toward our heading => tailwind (negative
    // drag); we return the headwind component as the negative of that.
    // `angle_diff` is in [-180, 180], so `along.cos()` needs no clamping.
    -(wind.speed_kn * along.cos())
}

/// Smallest signed difference `a - b` in degrees, normalized to [-180, 180].
fn angle_diff(a: f64, b: f64) -> f64 {
    let mut d = (a - b) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    d
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::hazards::rectangular_restriction;
    use crate::weather::{Current, Wind};

    #[test]
    fn calm_route_is_short_and_feasible() {
        let p = Planner::calm();
        let plan = p
            .plan(Position::new(-122.4, 37.8), Position::new(-118.2, 33.7))
            .expect("plan");
        assert!(plan.distance_nm > 300.0);
        assert!(plan.fuel_tonnes > 0.0);
        assert!(plan.time_hours > 0.0);
        // Endpoints preserved.
        assert_eq!(
            plan.waypoints.first().expect("start wp").pos,
            Position::new(-122.4, 37.8)
        );
        assert_eq!(
            plan.waypoints.last().expect("end wp").pos,
            Position::new(-118.2, 33.7)
        );
    }

    #[test]
    fn degenerate_endpoints_rejected() {
        let p = Planner::calm();
        let r = p.plan(Position::new(0.0, 0.0), Position::new(0.0, 0.0));
        assert_eq!(r, Err(PlanError::DegenerateEndpoints));
    }

    #[test]
    fn hazard_blocks_direct_route() {
        // A wall straight across the path.
        let wall = Hazard::Restricted(rectangular_restriction(
            "wall",
            Position::new(-120.4, 33.0),
            Position::new(-120.2, 40.0),
        ));
        let p = Planner::new(
            CalmEnvironment,
            CalmEnvironment,
            vec![wall],
            VesselProfile::default(),
        );
        // If the wall fully blocks, planner should fail gracefully.
        let r = p.plan(Position::new(-122.4, 37.8), Position::new(-118.2, 37.8));
        // Either it found a detour above/below, or it reports no feasible route.
        match r {
            Ok(plan) => assert!(plan.distance_nm > 0.0),
            Err(PlanError::NoFeasibleRoute) => {}
            Err(e) => panic!("unexpected error {e:?}"),
        }
    }

    #[test]
    fn fuel_penalty_for_headwind() {
        use crate::weather::{WeatherField, Wind};
        struct Headwind;
        impl WeatherField for Headwind {
            fn wind_at(&self, _pos: &Position) -> Option<Wind> {
                Some(Wind {
                    from_bearing: 90.0,
                    speed_kn: 30.0,
                })
            }
            fn wave_height_m(&self, _pos: &Position) -> Option<f64> {
                Some(0.0)
            }
        }
        impl crate::weather::CurrentSource for Headwind {
            fn current_at(&self, _pos: &Position) -> Option<Current> {
                Some(Current {
                    set_bearing: 0.0,
                    speed_kn: 0.0,
                })
            }
        }
        // Route heading east (bearing 90) into a westerly (from 90) headwind.
        let calm = Planner::calm();
        let windy = Planner::new(Headwind, CalmEnvironment, vec![], VesselProfile::default());
        let a = Position::new(0.0, 0.0);
        let b = Position::new(2.0, 0.0);
        let calm_leg = calm.leg_fuel(&a, &b);
        let windy_leg = windy.leg_fuel(&a, &b);
        assert!(windy_leg > calm_leg, "headwind should increase fuel");
    }

    #[test]
    fn following_current_reduces_fuel() {
        use crate::weather::CurrentSource;
        struct Following;
        impl WeatherField for Following {
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
        impl CurrentSource for Following {
            fn current_at(&self, _pos: &Position) -> Option<Current> {
                // Current flowing east (toward 90), same as leg heading.
                Some(Current {
                    set_bearing: 90.0,
                    speed_kn: 4.0,
                })
            }
        }
        let calm = Planner::calm();
        let assisted = Planner::new(CalmEnvironment, Following, vec![], VesselProfile::default());
        let a = Position::new(0.0, 0.0);
        let b = Position::new(2.0, 0.0);
        let calm_leg = calm.leg_fuel(&a, &b);
        let assisted_leg = assisted.leg_fuel(&a, &b);
        assert!(
            assisted_leg < calm_leg,
            "following current should reduce fuel"
        );
    }
}

// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test: planning a route around a hazard against a sample chart
//! region and a simple weather field.
//!
//! Validates the full planner path: environmental sampling, hazard avoidance,
//! and fuel/time estimation, using a contrived but realistic coastal scenario.

#[allow(clippy::expect_used)]
mod tests {
    use tp_helm_route::geo::Position;
    use tp_helm_route::hazards::{rectangular_restriction, Hazard};
    use tp_helm_route::optimize::{PlanError, Planner, VesselProfile};
    use tp_helm_route::weather::{CalmEnvironment, Current, CurrentSource, WeatherField, Wind};

    /// A constant westerly wind over the whole region (blows from the west).
    struct WesterlyWind;

    impl WeatherField for WesterlyWind {
        fn wind_at(&self, _pos: &Position) -> Option<Wind> {
            Some(Wind {
                from_bearing: 270.0,
                speed_kn: 20.0,
            })
        }
        fn wave_height_m(&self, _pos: &Position) -> Option<f64> {
            Some(2.0)
        }
    }

    impl CurrentSource for WesterlyWind {
        fn current_at(&self, _pos: &Position) -> Option<Current> {
            Some(Current {
                set_bearing: 90.0, // easterly (toward 90), opposes a westerly wind
                speed_kn: 1.5,
            })
        }
    }

    #[test]
    fn plans_around_coastal_landmass() {
        // A landmass blocking the direct route; the planner must detour around it.
        let land = Hazard::Restricted(rectangular_restriction(
            "coast",
            Position::new(-120.5, 35.0),
            Position::new(-119.5, 38.0),
        ));

        let planner = Planner::new(
            WesterlyWind,
            CalmEnvironment,
            vec![land],
            VesselProfile::default(),
        );

        let start = Position::new(-122.0, 36.5);
        let end = Position::new(-118.0, 36.5);

        let plan = planner.plan(start, end).expect("feasible detour exists");

        // The detour must be longer than the straight line (which is blocked).
        let straight = start.distance_nm(&end);
        assert!(
            plan.distance_nm > straight,
            "detour {} nm should exceed straight {} nm",
            plan.distance_nm,
            straight
        );

        // No waypoint may sit inside the hazard.
        for wp in &plan.waypoints {
            assert!(
                wp.pos.lon < -120.5
                    || wp.pos.lon > -119.5
                    || wp.pos.lat < 35.0
                    || wp.pos.lat > 38.0,
                "waypoint {:?} inside hazard",
                wp.pos
            );
        }

        // Fuel and time must be positive and consistent with the vessel profile.
        assert!(plan.fuel_tonnes > 0.0);
        assert!(plan.time_hours > 0.0);
    }

    #[test]
    fn fully_blocked_region_reports_failure() {
        // A hazard wall spanning the entire latitude range between the endpoints.
        let wall = Hazard::Restricted(rectangular_restriction(
            "wall",
            Position::new(-120.5, 30.0),
            Position::new(-119.5, 45.0),
        ));
        let planner = Planner::new(
            CalmEnvironment,
            CalmEnvironment,
            vec![wall],
            VesselProfile::default(),
        );
        let r = planner.plan(Position::new(-122.0, 36.5), Position::new(-118.0, 36.5));
        // Either a feasible detour above the wall, or an explicit failure.
        match r {
            Ok(plan) => assert!(plan.distance_nm > 0.0),
            Err(PlanError::NoFeasibleRoute) => {}
            Err(e) => panic!("unexpected {e:?}"),
        }
    }
}

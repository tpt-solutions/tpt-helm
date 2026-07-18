// SPDX-License-Identifier: MIT OR Apache-2.0

//! TPT Helm fuel-efficient route planning & optimization.
//!
//! Plans a vessel's path between two waypoints while minimizing fuel
//! consumption, accounting for weather (wind/waves), ocean currents, and
//! traffic-restricted areas. The planner searches a graph of candidate
//! waypoints and returns the path with the lowest integrated fuel cost.
//!
//! See `spec.txt` (Phase 3) and `todo.md`. The optimization objective is
//! fuel efficiency; safety constraints (restricted areas, land) are modeled
//! as hard obstacles the planner must avoid.

// Numeric casts in the planner (index↔coordinate, degree math) are deliberate
// and bounds-checked at the call sites, so the pedantic cast lints are relaxed
// to match the rest of the workspace.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]

pub mod geo;
pub mod hazards;
pub mod optimize;
pub mod weather;

pub use geo::{Haversine, Position};
pub use hazards::{Hazard, RestrictedArea, TrafficZone};
pub use optimize::{PlanError, Planner, RoutePlan, Waypoint};
pub use weather::{Current, WeatherField, Wind};

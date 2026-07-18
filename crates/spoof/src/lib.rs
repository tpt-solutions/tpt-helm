// SPDX-License-Identifier: MIT OR Apache-2.0

//! TPT Helm GPS spoofing / interference detection.
//!
//! Cross-checks the GPS-derived position fix against independent,
//! RF-independent navigation references — an Inertial Navigation System (INS)
//! and celestial (sextant/chronometer) fixes — to detect GPS spoofing and
//! interference. When the GPS position disagrees with these references by more
//! than their combined uncertainty, the [`Detector`] raises a spoofing
//! [`Alert`](detector::Alert) with an associated confidence score.
//!
//! See `spec.txt` (Phase 4) and `todo.md`. A commissioned independent security
//! review of this logic is still required before operational use
//! (see todo.md Phase 4).

pub mod celestial;
pub mod detector;
pub mod geo;
pub mod inertial;

pub use celestial::{CelestialCrossCheck, CelestialFix};
pub use detector::{
    Alert, CrossCheck, CrossCheckResult, Detector, DetectorConfig, ReferenceSource, Severity,
};
pub use geo::Position;
pub use inertial::{InertialCrossCheck, InertialFix};

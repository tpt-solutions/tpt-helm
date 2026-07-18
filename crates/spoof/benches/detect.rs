// SPDX-License-Identifier: MIT OR Apache-2.0

//! Benchmark: detection throughput across many evaluation cycles.
//!
//! Measures how cheaply the detector can cross-check a GPS fix against both
//! inertial and celestial references, on hot caches. Real deployments evaluate

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::semicolon_if_nothing_returned
)]
//! at the GPS fix rate (typically 1–10 Hz), so this is comfortably in budget.

use std::any::Any;

use criterion::{criterion_group, criterion_main, Criterion};
use tp_helm_spoof::celestial::{CelestialCrossCheck, CelestialFix};
use tp_helm_spoof::detector::{Detector, DetectorConfig};
use tp_helm_spoof::geo::Position;
use tp_helm_spoof::inertial::{InertialCrossCheck, InertialFix};

#[allow(clippy::unnecessary_wraps)]
fn payload<T: Any + 'static>(v: T) -> Option<Box<dyn Any>> {
    Some(Box::new(v))
}

fn bench_detect(c: &mut Criterion) {
    let detector = Detector::new(
        vec![Box::new(InertialCrossCheck), Box::new(CelestialCrossCheck)],
        DetectorConfig::default(),
    );
    let gps = Position::new(-122.0, 37.0);
    let ins = InertialFix::new(Position::new(-122.0001, 37.0001), 30.0);
    let cel = CelestialFix::new(Position::new(-122.004, 37.004));

    c.bench_function("evaluate_pair", |b| {
        b.iter(|| detector.evaluate(&gps, &[payload(ins), payload(cel)]));
    });
}

criterion_group!(benches, bench_detect);
criterion_main!(benches);

// SPDX-License-Identifier: MIT OR Apache-2.0

//! Benchmark: planning a long ocean voyage route.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::semicolon_if_nothing_returned
)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tp_helm_route::geo::Position;
use tp_helm_route::optimize::Planner;

#[allow(clippy::expect_used)]
fn bench_plan(c: &mut Criterion) {
    let planner = Planner::calm();
    let start = Position::new(-122.4, 37.8);
    let end = Position::new(-5.0, 36.0); // transatlantic-ish

    c.bench_function("plan_transoceanic", |b| {
        b.iter(|| {
            let plan = planner
                .plan(black_box(start), black_box(end))
                .expect("plan");
            black_box(plan.waypoints.len());
        });
    });
}

criterion_group!(benches, bench_plan);
criterion_main!(benches);

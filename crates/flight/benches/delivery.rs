// SPDX-License-Identifier: MIT OR Apache-2.0

//! Benchmark report delivery throughput across a connected link.

#![allow(clippy::cast_precision_loss, clippy::semicolon_if_nothing_returned)]

use criterion::{criterion_group, criterion_main, Criterion};
use tp_helm_flight::{LinkClient, LinkConfig, LinkStatus, Position, Transport, VesselIdentity};

struct AlwaysUp;
impl Transport for AlwaysUp {
    fn is_connected(&self) -> bool {
        true
    }
    fn send(&self, _: &tp_helm_flight::ShipStatusReport) -> LinkStatus {
        LinkStatus::Delivered
    }
}

fn bench_delivery(c: &mut Criterion) {
    c.bench_function("deliver_1000_reports", |b| {
        b.iter(|| {
            let mut client = LinkClient::new(
                AlwaysUp,
                LinkConfig {
                    queue_capacity: 2048,
                    vessel: VesselIdentity::new(1, 2),
                },
            );
            for i in 0..1000 {
                client.report_position(i, Position::new(-122.0, 37.0), 90.0, 12.0, 90.0, 0);
                let _ = client.tick();
            }
        })
    });
}

criterion_group!(benches, bench_delivery);
criterion_main!(benches);

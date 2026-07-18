// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(clippy::expect_used, clippy::unwrap_used)]

use criterion::{criterion_group, criterion_main, Criterion};
use tp_helm_ais::message::decode;
use tp_helm_ais::nmea::{parse_sentence, reassemble};

fn bench_decode(c: &mut Criterion) {
    let type1 = "15M67FC000G?ufbE`HqM5@0<0<0";
    c.bench_function("decode_type1", |b| {
        b.iter(|| {
            let _ = decode(type1);
        });
    });

    let type5_a = parse_sentence("!AIVDM,2,1,1,A,55M67FC00001M@<:V381T003`?R0T4PP0000001,0*00")
        .expect("valid");
    let type5_b = parse_sentence("!AIVDM,2,2,1,A,0000000000000000,0*17").expect("valid");
    let fa = type5_a.ais_fragment().expect("fragment");
    let fb = type5_b.ais_fragment().expect("fragment");
    c.bench_function("reassemble_and_decode_type5", |b| {
        b.iter(|| {
            let payload = reassemble(&[fa.clone(), fb.clone()]).expect("reassembles");
            let _ = decode(&payload);
        });
    });
}

criterion_group!(benches, bench_decode);
criterion_main!(benches);

// SPDX-License-Identifier: MIT OR Apache-2.0

//! Load benchmark: tessellating a large cell with many AIS targets.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::semicolon_if_nothing_returned,
    clippy::cast_lossless
)]
use std::collections::BTreeMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use geo::{Coord, LineString, Polygon};
use tp_helm_ecdis::render::overlay::{AisTarget, Overlay, OwnShip};
use tp_helm_ecdis::render::tessellate::{Viewport, WorldToScreen};
use tp_helm_ecdis::s52::Symbolizer;
use tp_helm_ecdis::s57::model::{
    BoundingBox, Chart, DatasetInfo, Feature, Spatial, SpatialPrimitive,
};

fn large_cell(target_count: usize) -> (Chart, Overlay, WorldToScreen) {
    let bounds = BoundingBox {
        west: -123.0,
        south: 37.0,
        east: -122.0,
        north: 38.0,
    };
    // Many small depth-area polygons to simulate a dense chart.
    let mut features = Vec::new();
    let mut spatial = Vec::new();
    let cells = 40;
    let mut idx = 0usize;
    for i in 0..cells {
        for j in 0..cells {
            let x0 = -123.0 + f64::from(i) * (1.0 / f64::from(cells));
            let y0 = 37.0 + f64::from(j) * (1.0 / f64::from(cells));
            let dx = 0.8 / f64::from(cells);
            let dy = 0.8 / f64::from(cells);
            let pts = vec![
                Coord { x: x0, y: y0 },
                Coord { x: x0 + dx, y: y0 },
                Coord {
                    x: x0 + dx,
                    y: y0 + dy,
                },
                Coord { x: x0, y: y0 + dy },
                Coord { x: x0, y: y0 },
            ];
            let sname = format!("S{idx}");
            spatial.push(Spatial {
                name: sname.clone(),
                primitive: SpatialPrimitive::Area(Polygon::new(LineString(pts), vec![])),
            });
            features.push(Feature {
                name: format!("F{idx}"),
                code: "DEPARE".into(),
                attributes: BTreeMap::default(),
                spatial_refs: vec![sname],
            });
            idx += 1;
        }
    }

    let mut overlay = Overlay::new();
    overlay.own_ship = Some(OwnShip {
        lon: -122.5,
        lat: 37.5,
        heading: 0.0,
    });
    for k in 0..target_count {
        overlay.targets.push(AisTarget {
            mmsi: 100_000_000 + k as u32,
            lon: -123.0 + (k % 100) as f64 * 0.01,
            lat: 37.0 + (k / 100) as f64 * 0.01,
            cog: Some((k % 360) as f32),
            sog: Some(10.0),
        });
    }

    let chart = Chart {
        metadata: DatasetInfo {
            bounds: Some(bounds),
            ..Default::default()
        },
        features,
        spatial,
    };
    let viewport = Viewport {
        width: 1920.0,
        height: 1080.0,
    };
    let proj = WorldToScreen::fit(bounds, viewport);
    (chart, overlay, proj)
}

fn bench_render_load(c: &mut Criterion) {
    let (chart, overlay, proj) = large_cell(10_000);
    let sym = Symbolizer::with_defaults();

    c.bench_function("tessellate_large_cell_10k_targets", |b| {
        b.iter(|| {
            let mut frame =
                tp_helm_ecdis::render::tessellate::tessellate(black_box(&chart), &proj, &sym);
            overlay.draw(&proj, &mut frame);
            black_box(frame.commands.len())
        })
    });
}

criterion_group!(benches, bench_render_load);
criterion_main!(benches);

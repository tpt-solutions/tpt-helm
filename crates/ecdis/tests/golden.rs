// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test: golden-image rendering of a known chart.
//!
//! Builds a small synthetic ENC with a coastline, a depth area, a buoy, and an
//! obstruction, tessellates it, rasterizes it with the headless CPU rasterizer,
//! and asserts the pixel checksum is stable. This is the golden-image
//! comparison required by Phase 2.

#![allow(clippy::expect_used, clippy::too_many_lines)]
use geo::{Coord, LineString, Point, Polygon};
use std::collections::BTreeMap;
use tp_helm_ecdis::render::overlay::{AisTarget, Overlay, OwnShip};
use tp_helm_ecdis::render::raster::Image;
use tp_helm_ecdis::render::tessellate::{Viewport, WorldToScreen};
use tp_helm_ecdis::s52::Symbolizer;
use tp_helm_ecdis::s57::model::{
    AttributeValue, BoundingBox, Chart, DatasetInfo, Feature, Spatial, SpatialPrimitive,
};

fn fixture_chart() -> Chart {
    let bounds = BoundingBox {
        west: -122.40,
        south: 37.75,
        east: -122.30,
        north: 37.85,
    };

    // Coastline polygon (land on the north-east side).
    let coast_pts = vec![
        Coord {
            x: -122.34,
            y: 37.82,
        },
        Coord {
            x: -122.30,
            y: 37.83,
        },
        Coord {
            x: -122.30,
            y: 37.85,
        },
        Coord {
            x: -122.36,
            y: 37.85,
        },
        Coord {
            x: -122.36,
            y: 37.82,
        },
        Coord {
            x: -122.34,
            y: 37.82,
        },
    ];
    let coast = Spatial {
        name: "S_COAST".into(),
        primitive: SpatialPrimitive::Area(Polygon::new(LineString(coast_pts), vec![])),
    };

    // Depth area (shallow) in the south-west.
    let depth_pts = vec![
        Coord {
            x: -122.40,
            y: 37.75,
        },
        Coord {
            x: -122.34,
            y: 37.75,
        },
        Coord {
            x: -122.34,
            y: 37.80,
        },
        Coord {
            x: -122.40,
            y: 37.80,
        },
        Coord {
            x: -122.40,
            y: 37.75,
        },
    ];
    let depth = Spatial {
        name: "S_DEPTH".into(),
        primitive: SpatialPrimitive::Area(Polygon::new(LineString(depth_pts), vec![])),
    };

    // A buoy (point).
    let buoy = Spatial {
        name: "S_BUOY".into(),
        primitive: SpatialPrimitive::Node(Point::new(-122.38, 37.78)),
    };

    // An obstruction (point).
    let obst = Spatial {
        name: "S_OBST".into(),
        primitive: SpatialPrimitive::Node(Point::new(-122.36, 37.77)),
    };

    let mut depth_attrs = BTreeMap::new();
    depth_attrs.insert("DRVAL1".into(), AttributeValue::Text("4".into()));

    let features = vec![
        Feature {
            name: "F_COAST".into(),
            code: "LNDARE".into(),
            attributes: BTreeMap::new(),
            spatial_refs: vec!["S_COAST".into()],
        },
        Feature {
            name: "F_DEPTH".into(),
            code: "DEPARE".into(),
            attributes: depth_attrs,
            spatial_refs: vec!["S_DEPTH".into()],
        },
        Feature {
            name: "F_BUOY".into(),
            code: "BUISGL".into(),
            attributes: BTreeMap::new(),
            spatial_refs: vec!["S_BUOY".into()],
        },
        Feature {
            name: "F_OBST".into(),
            code: "OBSTRN".into(),
            attributes: BTreeMap::new(),
            spatial_refs: vec!["S_OBST".into()],
        },
    ];

    Chart {
        metadata: DatasetInfo {
            bounds: Some(bounds),
            ..Default::default()
        },
        features,
        spatial: vec![coast, depth, buoy, obst],
    }
}

#[test]
fn golden_image_is_stable() {
    let chart = fixture_chart();
    let viewport = Viewport {
        width: 640.0,
        height: 480.0,
    };
    let proj = WorldToScreen::fit(chart.metadata.bounds.expect("bounds"), viewport);
    let sym = Symbolizer::with_defaults();

    let mut frame = tp_helm_ecdis::render::tessellate::tessellate(&chart, &proj, &sym);

    // Overlay: own ship at chart center, two AIS contacts.
    let mut overlay = Overlay::new();
    overlay.own_ship = Some(OwnShip {
        lon: -122.35,
        lat: 37.80,
        heading: 45.0,
    });
    overlay.targets.push(AisTarget {
        mmsi: 123_456_789,
        lon: -122.37,
        lat: 37.79,
        cog: Some(120.0),
        sog: Some(8.0),
    });
    overlay.targets.push(AisTarget {
        mmsi: 987_654_321,
        lon: -122.33,
        lat: 37.81,
        cog: None,
        sog: None,
    });
    overlay.draw(&proj, &mut frame);

    let mut img = Image::new(640, 480);
    img.draw(&frame);
    let checksum = img.checksum();

    // Snapshot recorded from the reference render. If the engine changes in a
    // way that alters output, this must be updated deliberately (not blindly).
    assert_eq!(
        checksum, 0xc640_d1ba_ff2e_724e,
        "golden image checksum changed — verify rendering output before updating"
    );
}

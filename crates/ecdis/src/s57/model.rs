// SPDX-License-Identifier: MIT OR Apache-2.0

//! S-57 spatial and feature object model (IHO S-57 Edition 3.1).
//!
//! This module models the in-memory representation of an ENC cell after it has
//! been decoded from the ISO 8211 base file by the [`crate::s57::parser`]. The
//! model deliberately follows the S-57 conceptual schema (feature objects with
//! attributes, spatial objects with geometry, and vector record relationships)
//! at an architectural level. No GPL-licensed implementation (e.g. OpenCPN) is
//! copied or ported — the schema is reconstructed from the public IHO standard.

use std::collections::BTreeMap;

use geo::{Coord, LineString, Point, Polygon};

/// A geographically referenced ENC cell (one `.000` file).
#[derive(Debug, Clone, PartialEq)]
pub struct Chart {
    /// Dataset descriptive metadata from the `DSID` record.
    pub metadata: DatasetInfo,
    /// All feature objects keyed by their long name (e.g. `BUISGL`).
    pub features: Vec<Feature>,
    /// All spatial objects referenced by features.
    pub spatial: Vec<Spatial>,
}

/// Metadata drawn from the S-57 `DSID` (dataset identification) record.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DatasetInfo {
    /// Producer agency code.
    pub agency: String,
    /// Dataset file name (e.g. `US5WA0001`).
    pub file_name: String,
    /// Volume / edition identifier.
    pub edition: String,
    /// Integer edition number.
    pub edition_number: u32,
    /// Integer update number.
    pub update_number: u32,
    /// Bounding box of the cell, if present in `DSID`.
    pub bounds: Option<BoundingBox>,
}

/// Axis-aligned geographic extent in WGS84 degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// A feature object record (e.g. a buoy, a depth area, a coastline).
///
/// Features carry semantic attributes and reference one or more spatial objects
/// for their geometry. `attributes` is a map of S-57 attribute acronyms to
/// decoded values.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// Record name (hex identifier from the `FRID` record).
    pub name: String,
    /// Feature object code (e.g. `LIGHTS`, `DEPARE`, `SLCONS`).
    pub code: FeatureCode,
    /// Decoded attribute values keyed by acronym (`HEIGHT`, `COLOUR`, ...).
    pub attributes: BTreeMap<String, AttributeValue>,
    /// Names of the spatial objects this feature is built from.
    pub spatial_refs: Vec<String>,
}

/// S-57 feature object code (FOC).
pub type FeatureCode = String;

/// A decoded S-57 attribute value.
///
/// The standard mixes enumerations, free text, and numeric quantities with
/// units; we keep the raw, typed value here and let the S-52 symbology engine
/// interpret it.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    /// Integer enumeration code (from an S-57 code list).
    Enum(i64),
    /// Free text (names, descriptions).
    Text(String),
    /// Floating-point measurement in the attribute's implicit unit.
    Float(f64),
    /// Integer measurement.
    Int(i64),
    /// List of enum codes (e.g. `CATLIT` repeatable).
    EnumList(Vec<i64>),
}

/// A spatial object record (vector geometry).
#[derive(Debug, Clone, PartialEq)]
pub struct Spatial {
    /// Record name (hex identifier from the `VRID` record).
    pub name: String,
    /// Spatial primitive type.
    pub primitive: SpatialPrimitive,
}

/// S-57 vector spatial primitive types.
#[derive(Debug, Clone, PartialEq)]
pub enum SpatialPrimitive {
    /// Node (isolated point feature).
    Node(Point2D),
    /// Edge (line segment between two nodes).
    Edge(Line2D),
    /// Area (closed polygon).
    Area(Polygon2D),
}

/// A geographic point in WGS84 degrees.
pub type Point2D = Point<f64>;

/// An ordered set of coordinates forming a polyline.
pub type Line2D = LineString<f64>;

/// A polygon with optional interior rings.
pub type Polygon2D = Polygon<f64>;

/// Convenience constructor for a coordinate in degrees.
#[must_use]
pub fn coord(lon: f64, lat: f64) -> Coord<f64> {
    Coord { x: lon, y: lat }
}

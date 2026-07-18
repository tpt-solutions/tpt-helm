// SPDX-License-Identifier: MIT OR Apache-2.0

//! S-57 Electronic Navigational Chart (ENC) support for TPT Helm.
//!
//! This module models the S-57 conceptual schema (feature and spatial objects)
//! and decodes ENC cells from their ISO 8211 base files. The schema is
//! reconstructed from the public IHO S-57 Edition 3.1 standard; no
//! GPL-licensed implementation is copied or ported.

pub mod model;
pub mod parser;

pub use model::{
    AttributeValue, BoundingBox, Chart, DatasetInfo, Feature, Spatial, SpatialPrimitive,
};
pub use parser::{parse_bytes, parse_reader, S57Error};

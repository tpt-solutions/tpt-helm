// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal ISO 8211 / S-57 base file reader.
//!
//! S-57 Edition 3.1 cells are exchanged as ISO 8211 encoded files. The format is
//! a self-describing, record-oriented, fixed- and variable-length field
//! container. This reader implements just enough of ISO 8211 to extract the
//! `DSID`, `FRID`/`FOID`/`ATTR`/`ATTF`/`FSPT`, and `VRID`/`SG2D`/`VCID` records
//! that make up a chart. It is intentionally small and dependency-free so it can
//! run on embedded marine PCs without a full geospatial stack.

use std::collections::BTreeMap;
use std::io::Read;

use super::model::{AttributeValue, Chart, DatasetInfo, Feature, Spatial, SpatialPrimitive};

/// Errors raised while decoding an S-57 base file.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum S57Error {
    /// The file did not begin with a valid ISO 8211 leader.
    #[error("not an ISO 8211 file (missing leader)")]
    NotIso8211,
    /// A data descriptive record (DDR) could not be parsed.
    #[error("invalid data descriptive record: {0}")]
    BadDdr(String),
    /// A data record (DR) could not be parsed.
    #[error("invalid data record: {0}")]
    BadDr(String),
    /// The mandatory `DSID` record was missing.
    #[error("missing DSID dataset identification record")]
    MissingDsid,
    /// An integer field held non-ASCII digits.
    #[error("non-numeric value in integer field")]
    NonNumeric,
    /// Unexpected end of input.
    #[error("unexpected end of file")]
    UnexpectedEof,
}

/// Parse a complete S-57 base file from a reader.
///
/// # Errors
/// Returns [`S57Error`] if the byte stream is not a well-formed ISO 8211 file or
/// is missing required records.
pub fn parse_reader<R: Read>(mut reader: R) -> Result<Chart, S57Error> {
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| S57Error::UnexpectedEof)?;
    parse_bytes(&bytes)
}

/// Parse a complete S-57 base file from an in-memory byte buffer.
///
/// # Errors
/// See [`parse_reader`].
pub fn parse_bytes(bytes: &[u8]) -> Result<Chart, S57Error> {
    let mut cursor = 0usize;
    let ddr = parse_ddr(bytes, &mut cursor)?;
    let mut dsid: Option<DatasetInfo> = None;
    let mut features: Vec<Feature> = Vec::new();
    let mut spatial: Vec<Spatial> = Vec::new();

    while cursor < bytes.len() {
        let Some(dr) = parse_dr(bytes, &mut cursor, &ddr)? else {
            break;
        };
        match dr.tag.as_str() {
            "DSID" => dsid = Some(parse_dsid(&dr)),
            "FRID" => features.push(parse_frid(&dr)),
            "VRID" => spatial.push(parse_vrid(&dr)),
            // Other record types (e.g. `VRPT`, `SG3D`) are not yet modeled.
            _ => {}
        }
    }

    let metadata = dsid.ok_or(S57Error::MissingDsid)?;
    Ok(Chart {
        metadata,
        features,
        spatial,
    })
}

/// Parse the DDR, building the field/subfield layout used to decode DRs.
fn parse_ddr(bytes: &[u8], cursor: &mut usize) -> Result<Ddr, S57Error> {
    let leader = Leader::parse(bytes, cursor)?;
    if leader.format != b' ' {
        return Err(S57Error::BadDdr("expected ISO 8211 leader".into()));
    }
    let base = *cursor + leader.record_length as usize;
    let field_area_start = leader.field_area_base as usize;
    *cursor += leader.record_length as usize;

    let mut fields: Vec<FieldDef> = Vec::new();
    let mut pos = 0usize;
    while pos < field_area_start - 1 {
        let tag = std::str::from_utf8(&bytes[base + pos..base + pos + 4])
            .map_err(|_| S57Error::BadDdr("bad field tag".into()))?
            .to_string();
        let field_len = u16::from_be_bytes([bytes[base + pos + 4], bytes[base + pos + 5]]);
        let field_pos = u16::from_be_bytes([bytes[base + pos + 6], bytes[base + pos + 7]]);
        fields.push(FieldDef {
            tag,
            field_len,
            field_pos,
        });
        pos += 8;
    }
    Ok(Ddr { fields })
}

/// Parse one data record, returning its decoded fields.
fn parse_dr<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    ddr: &Ddr,
) -> Result<Option<DataRecord<'a>>, S57Error> {
    if *cursor + 24 > bytes.len() {
        return Ok(None);
    }
    let leader = Leader::parse(bytes, cursor)?;
    if leader.record_length == 0 {
        return Ok(None);
    }
    let rec_start = *cursor;
    let base = rec_start + leader.field_area_base as usize;
    let field_area_end = rec_start + leader.record_length as usize;
    *cursor += leader.record_length as usize;

    // The directory of a DR lives after the 12-byte field area locator.
    let dir_start = rec_start + 12;
    let dir_end = rec_start + leader.field_area_base as usize;

    let mut fields: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut pos = dir_start;
    while pos + 12 <= dir_end {
        let tag = std::str::from_utf8(&bytes[pos..pos + 4])
            .map_err(|_| S57Error::BadDr("bad field tag".into()))?
            .to_string();
        let field_len = u16::from_be_bytes([bytes[pos + 4], bytes[pos + 5]]) as usize;
        let field_pos = u16::from_be_bytes([bytes[pos + 6], bytes[pos + 7]]) as usize;
        let fstart = base + field_pos;
        let fend = if field_len > 0 {
            fstart + field_len
        } else {
            field_area_end
        };
        fields.insert(tag, (fstart, fend.min(field_area_end)));
        pos += 12;
    }

    // Identify the record type from the first directory entry tag.
    let tag = ddr
        .fields
        .first()
        .map(|f| f.tag.clone())
        .unwrap_or_default();
    Ok(Some(DataRecord { tag, fields, bytes }))
}

/// Decode the `DSID` record into dataset metadata.
fn parse_dsid(dr: &DataRecord<'_>) -> DatasetInfo {
    let mut info = DatasetInfo::default();
    if let Some((s, e)) = dr.field("DSID") {
        // DSID subfields: RCID, FILE, LFIL, AGEN, FIDN, FIDS, LUPD, UADT,
        // ISDT, STED, PRSP, PSDN, PRED, PROF, VERD, DVSN, EDTN, UPDN, UADT, MXSH.
        let sub = dr.subfields(s, e, &["FILE", "AGEN", "EDTN", "UPDN"]);
        info.file_name = sub.get("FILE").map(|v| clean(v)).unwrap_or_default();
        info.agency = sub.get("AGEN").map(|v| clean(v)).unwrap_or_default();
        info.edition = sub.get("EDTN").map(|v| clean(v)).unwrap_or_default();
        info.update_number = sub
            .get("UPDN")
            .and_then(|v| clean(v).parse().ok())
            .unwrap_or(0);
    }
    info
}

/// Decode a `FRID` (feature record) into a [`Feature`].
fn parse_frid(dr: &DataRecord<'_>) -> Feature {
    let mut feature = Feature {
        name: String::new(),
        code: String::new(),
        attributes: BTreeMap::new(),
        spatial_refs: Vec::new(),
    };
    if let Some((s, e)) = dr.field("FRID") {
        let sub = dr.subfields(s, e, &["RCID", "FIDN", "FRCN"]);
        feature.name = sub
            .get("RCID")
            .or_else(|| sub.get("FIDN"))
            .map(|v| clean(v))
            .unwrap_or_default();
        feature.code = sub.get("FRCN").map(|v| clean(v)).unwrap_or_default();
    }
    // Attributes (ATTF) — free-text attribute/value pairs.
    if let Some((s, e)) = dr.field("ATTF") {
        feature.attributes = parse_attf(dr.slice(s, e));
    }
    // Spatial reference (FSPT) — list of {NAME, ...}.
    if let Some((s, e)) = dr.field("FSPT") {
        feature.spatial_refs = parse_fspt(dr.slice(s, e));
    }
    feature
}

/// Parse `ATTF` subfield bytes into a map of attribute values.
///
/// Each attribute entry is `ATTL` (code), `ATVL` (value), repeated.
fn parse_attf(bytes: &[u8]) -> BTreeMap<String, AttributeValue> {
    let mut out = BTreeMap::new();
    let parts = split_units(bytes);
    let mut i = 0;
    while i + 1 < parts.len() {
        let attl = clean(std::str::from_utf8(parts[i]).unwrap_or(""));
        let atvl = clean(std::str::from_utf8(parts[i + 1]).unwrap_or(""));
        out.insert(attl, AttributeValue::Text(atvl));
        i += 2;
    }
    out
}

/// Parse `FSPT` subfield bytes into a list of spatial record names.
fn parse_fspt(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let parts = split_units(bytes);
    for p in parts {
        if p.is_empty() {
            continue;
        }
        let s = clean(std::str::from_utf8(p).unwrap_or(""));
        if !s.is_empty() {
            out.push(s);
        }
    }
    out
}

/// Decode a `VRID` (spatial record) into a [`Spatial`].
fn parse_vrid(dr: &DataRecord<'_>) -> Spatial {
    let mut spatial = Spatial {
        name: String::new(),
        primitive: SpatialPrimitive::Node(geo::Point::new(0.0, 0.0)),
    };
    if let Some((s, e)) = dr.field("VRID") {
        let sub = dr.subfields(s, e, &["RCID", "VRPT", "PRIM"]);
        spatial.name = sub
            .get("RCID")
            .or_else(|| sub.get("VRPT"))
            .map(|v| clean(v))
            .unwrap_or_default();
    }
    match dr.fields.get("SG2D") {
        Some(&(s, e)) => {
            let pts = parse_sg2d(dr.slice(s, e));
            if pts.len() >= 3 {
                use geo::Polygon;
                spatial.primitive =
                    SpatialPrimitive::Area(Polygon::new(geo::LineString(pts.clone()), vec![]));
            } else {
                spatial.primitive = SpatialPrimitive::Edge(geo::LineString(pts));
            }
        }
        None => {
            if let Some(&(s, e)) = dr.fields.get("SG3D") {
                let pts = parse_sg2d(dr.slice(s, e));
                spatial.primitive = SpatialPrimitive::Edge(geo::LineString(pts));
            }
        }
    }
    spatial
}

/// Decode an `SG2D`/`SG3D` coordinate list into a vector of 2D points.
///
/// Each coordinate is `{Y (lat) : 4 bytes BE, X (lon) : 4 bytes BE}` in units of
/// 1/100_000_000 degrees.
fn parse_sg2d(bytes: &[u8]) -> Vec<geo::Coord<f64>> {
    const UNIT: f64 = 1.0 / 100_000_000.0;
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        let lat =
            i32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]) as f64 * UNIT;
        let lon = i32::from_be_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]])
            as f64
            * UNIT;
        out.push(geo::Coord { x: lon, y: lat });
        i += 8;
    }
    out
}

/// Strip leading/trailing NUL, spaces, and ISO 8211 field terminators.
fn clean(s: &str) -> String {
    s.trim_matches(|c: char| {
        c == '\0' || c == ' ' || c == '\u{1f}' || c == '\u{1e}' || c == '\u{1d}'
    })
    .to_string()
}

/// Split an ISO 8211 unit-separated byte slice into its constituent units.
fn split_units(bytes: &[u8]) -> Vec<&[u8]> {
    // Unit terminator is 0x1F; field terminator 0x1E. We split on 0x1F.
    bytes.split(|&b| b == 0x1F || b == 0x1E).collect()
}

/// ISO 8211 leader (first 24 bytes of every record).
#[derive(Debug, Clone, Copy)]
struct Leader {
    record_length: u32,
    format: u8,
    field_area_base: u16,
}

impl Leader {
    fn parse(bytes: &[u8], cursor: &mut usize) -> Result<Self, S57Error> {
        if *cursor + 24 > bytes.len() {
            return Err(S57Error::UnexpectedEof);
        }
        let rec_len = std::str::from_utf8(&bytes[*cursor..*cursor + 5])
            .map_err(|_| S57Error::NotIso8211)?
            .parse::<u32>()
            .map_err(|_| S57Error::NotIso8211)?;
        let format = bytes[*cursor + 6];
        let field_area_base = std::str::from_utf8(&bytes[*cursor + 12..*cursor + 17])
            .map_err(|_| S57Error::BadDdr("leader".into()))?
            .parse::<u16>()
            .map_err(|_| S57Error::BadDdr("leader base".into()))?;
        Ok(Leader {
            record_length: rec_len,
            format,
            field_area_base,
        })
    }
}

/// A DDR field definition (tag + location).
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FieldDef {
    tag: String,
    field_len: u16,
    field_pos: u16,
}

/// Parsed DDR.
#[derive(Debug, Clone)]
struct Ddr {
    fields: Vec<FieldDef>,
}

/// A decoded data record with located fields.
#[derive(Debug)]
struct DataRecord<'a> {
    tag: String,
    fields: BTreeMap<String, (usize, usize)>,
    bytes: &'a [u8],
}

impl<'a> DataRecord<'a> {
    fn field(&self, name: &str) -> Option<(usize, usize)> {
        self.fields.get(name).copied()
    }

    fn slice(&self, start: usize, end: usize) -> &'a [u8] {
        &self.bytes[start..end]
    }

    /// Decode the named subfields from a field's byte range, splitting on the
    /// ISO 8211 unit terminator and pairing [tag, value] entries.
    fn subfields(&self, start: usize, end: usize, names: &[&str]) -> BTreeMap<String, String> {
        let bytes = &self.bytes[start..end];
        let units: Vec<&[u8]> = split_units(bytes);
        let mut out = BTreeMap::new();
        let mut i = 0;
        while i + 1 < units.len() {
            let tag = std::str::from_utf8(units[i])
                .unwrap_or("")
                .trim()
                .to_string();
            let val = std::str::from_utf8(units[i + 1]).unwrap_or("").to_string();
            if names.contains(&tag.as_str()) || names.is_empty() {
                out.insert(tag, val);
            }
            i += 2;
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_attf_pairs() {
        // Simulated ATTF: ATTL="HEIGHT" ATVL="12" ATTL="COLOUR" ATVL="1"
        let raw: Vec<u8> = b"HEIGHT\x1f12\x1fCOLOUR\x1f1\x1f".to_vec();
        let attrs = parse_attf(&raw);
        assert_eq!(
            attrs.get("HEIGHT"),
            Some(&AttributeValue::Text("12".into()))
        );
        assert_eq!(attrs.get("COLOUR"), Some(&AttributeValue::Text("1".into())));
    }

    #[test]
    fn parse_sg2d_units() {
        // One coordinate: lat = 1.0 deg, lon = 2.0 deg (in 1e-8 units).
        let lat = (1.0 * 100_000_000.0) as i32;
        let lon = (2.0 * 100_000_000.0) as i32;
        let raw = lat.to_be_bytes().to_vec();
        let mut raw = raw;
        raw.extend_from_slice(&lon.to_be_bytes());
        let pts = parse_sg2d(&raw);
        assert_eq!(pts.len(), 1);
        assert!((pts[0].y - 1.0).abs() < 1e-6);
        assert!((pts[0].x - 2.0).abs() < 1e-6);
    }
}

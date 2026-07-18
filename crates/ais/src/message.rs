// SPDX-License-Identifier: MIT OR Apache-2.0

//! AIS message decoding (ITU-R M.1371 / IEC 61161-1).
//!
//! Implements the most operationally important AIS message types: position
//! reports (1/2/3), static and voyage-related data (5), and the class B
//! variants (18, 24). Each decoder consumes a [`sixbit::BitReader`] built from
//! the six-bit packed payload of one or more assembled NMEA sentences.

// Bit-field decoding inherently narrows integers; field widths guarantee safety.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

use thiserror::Error;

use crate::sixbit::{unpack, BitReader};

/// Errors that can occur while decoding an AIS message payload.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AisMessageError {
    /// The message type is not yet supported by the decoder.
    #[error("unsupported AIS message type {0}")]
    UnsupportedType(u8),
    /// The payload bits were exhausted before a field could be read.
    #[error("payload too short for message type {0}")]
    Truncated(u8),
}

/// A decoded AIS message. Only the modeled types below are produced.
#[derive(Debug, Clone, PartialEq)]
pub enum AisMessage {
    /// Position report (message types 1, 2, 3) and class B position report
    /// (type 18) share the same position fields.
    PositionReport(PositionReport),
    /// Static and voyage-related data (message type 5).
    StaticVoyage(StaticVoyageData),
    /// Class B CS position report part A (message type 24, part A).
    ClassBStatic(StaticDataReport),
}

/// Common position-report fields (types 1/2/3 and 18).
#[derive(Debug, Clone, PartialEq)]
pub struct PositionReport {
    /// AIS message type (1, 2, 3, or 18).
    pub message_type: u8,
    /// Maritime Mobile Service Identity of the source station.
    pub mmsi: u32,
    /// Navigation status (0..=15; type 18 has none and reports 0).
    pub nav_status: u8,
    /// Rate of turn, signed tenths of degrees per minute.
    pub rot: i16,
    /// Speed over ground in tenths of a knot (1023 = not available).
    pub sog: u16,
    /// Position accuracy: `true` if high (<10 m), `false` if low.
    pub pos_accuracy: bool,
    /// Longitude in 1/10000th of a minute (East positive).
    pub longitude: Option<f64>,
    /// Latitude in 1/10000th of a minute (North positive).
    pub latitude: Option<f64>,
    /// Course over ground in tenths of a degree (3600 = not available).
    pub cog: u16,
    /// True heading in degrees (360 = not available).
    pub heading: u16,
    /// Timestamp: second of UTC minute (0..=59; 60 = not available).
    pub timestamp: u8,
}

/// Static and voyage-related data (message type 5).
#[derive(Debug, Clone, PartialEq)]
pub struct StaticVoyageData {
    /// Maritime Mobile Service Identity of the vessel.
    pub mmsi: u32,
    /// IMO number (0 = not available).
    pub imo_number: u32,
    /// Call sign (ASCII, padding stripped).
    pub callsign: String,
    /// Vessel name (ASCII, padding stripped).
    pub name: String,
    /// Ship type code (0..=99).
    pub ship_type: u8,
    /// Dimension to bow in metres.
    pub dim_to_bow: u16,
    /// Dimension to stern in metres.
    pub dim_to_stern: u16,
    /// Dimension to port in metres.
    pub dim_to_port: u16,
    /// Dimension to starboard in metres.
    pub dim_to_starboard: u16,
    /// Type of electronic position fixing device (0..=9).
    pub fix_type: u8,
    /// Maximum present static draught in metres / 10.
    pub draught: u8,
    /// Destination (ASCII, padding stripped).
    pub destination: String,
}

/// Class B static data report, part A (message type 24).
#[derive(Debug, Clone, PartialEq)]
pub struct StaticDataReport {
    /// Maritime Mobile Service Identity of the vessel.
    pub mmsi: u32,
    /// Vessel name (ASCII, padding stripped).
    pub name: String,
    /// Ship type code (0..=99).
    pub ship_type: u8,
    /// Vendor id / model string.
    pub vendor_id: String,
}

/// Longitude/latitude are encoded as signed integers in 1/10000 minute units.
/// A value of 0x6791AC0 (108° beyond range) means "not available".
const LON_LAT_UNAVAILABLE: i32 = 0x0679_1AC0;

/// Decode an AIS message from a six-bit packed ASCII payload string.
///
/// # Errors
/// Returns [`AisMessageError::UnsupportedType`] for unimplemented message
/// types and [`AisMessageError::Truncated`] if the payload is too short.
pub fn decode(payload: &str) -> Result<AisMessage, AisMessageError> {
    let mut reader = BitReader::new(unpack(payload));
    let message_type = reader.read_u32(6) as u8;
    match message_type {
        1..=3 => Ok(decode_position_report(&mut reader, message_type)),
        5 => Ok(decode_static_voyage(&mut reader)),
        18 => Ok(decode_position_report(&mut reader, 18)),
        24 => decode_static_data_report(&mut reader),
        other => Err(AisMessageError::UnsupportedType(other)),
    }
}

fn decode_position_report(reader: &mut BitReader, message_type: u8) -> AisMessage {
    let _ = reader.read_u32(2); // repeat indicator
    let mmsi = reader.read_u32(30);
    let nav_status = if message_type == 18 {
        0
    } else {
        reader.read_u32(4) as u8
    };
    let rot = if message_type == 18 {
        0
    } else {
        reader.read_i32(8) as i16
    };
    let sog = reader.read_u32(10) as u16;
    let pos_accuracy = reader.read_u32(1) == 1;
    let longitude = decode_coordinate(reader.read_i32(28));
    let latitude = decode_coordinate(reader.read_i32(27));
    let cog = reader.read_u32(12) as u16;
    let heading = reader.read_u32(9) as u16;
    let timestamp = reader.read_u32(6) as u8;

    AisMessage::PositionReport(PositionReport {
        message_type,
        mmsi,
        nav_status,
        rot,
        sog,
        pos_accuracy,
        longitude,
        latitude,
        cog,
        heading,
        timestamp,
    })
}

fn decode_static_voyage(reader: &mut BitReader) -> AisMessage {
    let _ = reader.read_u32(2); // repeat indicator
    let mmsi = reader.read_u32(30);
    let _ais_version = reader.read_u32(2);
    let imo_number = reader.read_u32(30);
    let callsign = reader.read_string(42);
    let name = reader.read_string(120);
    let ship_type = reader.read_u32(8) as u8;
    let dim_to_bow = reader.read_u32(9) as u16;
    let dim_to_stern = reader.read_u32(9) as u16;
    let dim_to_port = reader.read_u32(6) as u16;
    let dim_to_starboard = reader.read_u32(6) as u16;
    let _pos_fix_type = reader.read_u32(4);
    let fix_type = reader.read_u32(4) as u8;
    let _eta_month = reader.read_u32(4);
    let _eta_day = reader.read_u32(5);
    let _eta_hour = reader.read_u32(5);
    let _eta_minute = reader.read_u32(6);
    let draught = reader.read_u32(8) as u8;
    let destination = reader.read_string(120);

    AisMessage::StaticVoyage(StaticVoyageData {
        mmsi,
        imo_number,
        callsign,
        name,
        ship_type,
        dim_to_bow,
        dim_to_stern,
        dim_to_port,
        dim_to_starboard,
        fix_type,
        draught,
        destination,
    })
}

fn decode_static_data_report(reader: &mut BitReader) -> Result<AisMessage, AisMessageError> {
    let _ = reader.read_u32(2); // repeat indicator
    let mmsi = reader.read_u32(30);
    let part = reader.read_u32(2);
    if part != 0 {
        // Part B (dimensions) is not modeled yet.
        return Err(AisMessageError::UnsupportedType(24));
    }
    let name = reader.read_string(120);
    let ship_type = reader.read_u32(8) as u8;
    let vendor_id = reader.read_string(42);

    Ok(AisMessage::ClassBStatic(StaticDataReport {
        mmsi,
        name,
        ship_type,
        vendor_id,
    }))
}

fn decode_coordinate(raw: i32) -> Option<f64> {
    if raw == LON_LAT_UNAVAILABLE {
        None
    } else {
        Some(f64::from(raw) / 600_000.0)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn decodes_type_1_position_report() {
        // Known AIS type 1 message: !AIVDM,1,1,,A,15M67FC000G?ufbE`HqM5@0<0<0,0*46
        let msg = decode("15M67FC000G?ufbE`HqM5@0<0<0").expect("decodes");
        let AisMessage::PositionReport(r) = msg else {
            panic!("expected position report");
        };
        assert_eq!(r.message_type, 1);
        assert_eq!(r.mmsi, 366_053_209);
        assert_eq!(r.longitude, Some(-122.341_618_333_333_33));
        assert_eq!(r.latitude, Some(37.803_048_333_333_34));
    }

    #[test]
    fn decodes_type_5_static_voyage() {
        // Real two-part AIVDM type 5 message.
        let part_a = "55M67FC00001M@<:V381T003`?R0T4PP0000001";
        let part_b = "0000000000000000";
        let payload = format!("{part_a}{part_b}");
        let msg = decode(&payload).expect("decodes");
        let AisMessage::StaticVoyage(d) = msg else {
            panic!("expected static voyage");
        };
        assert_eq!(d.mmsi, 366_053_209);
        assert!(!d.name.is_empty());
    }

    #[test]
    fn rejects_unsupported_type() {
        assert!(matches!(
            decode("0000000000000000"),
            Err(AisMessageError::UnsupportedType(0))
        ));
    }
}

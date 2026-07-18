// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared message schema between TPT Helm (ship) and tpt-flight-control (port).
//!
//! These types are the wire contract for the satellite link. They are
//! stable, versioned, and serialize to a compact, deterministic form (the link
//! layer signs and encrypts the serialized bytes — see [`crate::client`]).

use serde::{Deserialize, Serialize};

/// A geographic position in degrees (WGS84).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// Longitude in degrees, -180..180.
    pub lon: f64,
    /// Latitude in degrees, -90..90.
    pub lat: f64,
}

impl Position {
    /// Construct a position.
    #[must_use]
    pub fn new(lon: f64, lat: f64) -> Self {
        Self { lon, lat }
    }
}

/// Identity of the reporting vessel, used by the port to route the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VesselIdentity {
    /// IMO number (7 digits) of the vessel.
    pub imo_number: u32,
    /// MMSI of the vessel's primary AIS transmitter.
    pub mmsi: u32,
}

impl VesselIdentity {
    /// Construct a vessel identity.
    #[must_use]
    pub fn new(imo_number: u32, mmsi: u32) -> Self {
        Self { imo_number, mmsi }
    }
}

/// A position/status report sent from the ship to the port scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShipStatusReport {
    /// Schema version, currently `1`.
    pub schema_version: u8,
    /// Monotonic sequence number; gaps indicate dropped reports.
    pub sequence: u64,
    /// Report generation time, Unix epoch seconds (UTC).
    pub report_time_epoch_s: u64,
    /// Vessel identity.
    pub vessel: VesselIdentity,
    /// Current position fix.
    pub position: Position,
    /// Course over ground, degrees true (0..360).
    pub cog_deg: f64,
    /// Speed over ground, knots.
    pub sog_kn: f64,
    /// Heading, degrees true (360 = not available).
    pub heading_deg: f64,
    /// Navigation status code (0..=15, per AIS).
    pub nav_status: u8,
}

impl ShipStatusReport {
    /// The current schema version.
    pub const SCHEMA_VERSION: u8 = 1;

    /// Build a report, assigning the schema version automatically.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sequence: u64,
        report_time_epoch_s: u64,
        vessel: VesselIdentity,
        position: Position,
        cog_deg: f64,
        sog_kn: f64,
        heading_deg: f64,
        nav_status: u8,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            sequence,
            report_time_epoch_s,
            vessel,
            position,
            cog_deg,
            sog_kn,
            heading_deg,
            nav_status,
        }
    }
}

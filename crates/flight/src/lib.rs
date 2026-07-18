// SPDX-License-Identifier: MIT OR Apache-2.0

//! TPT Helm ↔ tpt-flight-control integration (Helm side).
//!
//! Reports ship position and status to the port-side scheduling system
//! (`tpt-flight-control`) over a satellite (Starlink) link. The link is
//! intermittent, so reports are queued and retried offline until acknowledged.
//!
//! See `spec.txt` (Phase 4 of the TPT Fulcrum strategy) and `todo.md` Phase 6.
//! The wire schema ([`schema`]) is shared with `tpt-flight-control`. A security
//! review of the satellite link (auth + encryption) is required before
//! operational use (see `docs/security/flight-control-link.md`).

pub mod client;
pub mod queue;
pub mod schema;

pub use client::{LinkClient, LinkConfig, LinkStatus, MockTransport, ReportOutcome, Transport};
pub use queue::{QueuedReport, ReportQueue};
pub use schema::{Position, ShipStatusReport, VesselIdentity};

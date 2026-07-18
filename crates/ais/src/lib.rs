// SPDX-License-Identifier: MIT OR Apache-2.0

//! TPT Helm AIS parser.
//!
//! Decodes ship position and identification data from AIS/NMEA 0183 feeds.
//! See `spec.txt` (Phase 1) and `todo.md` for the roadmap.

pub mod message;
pub mod nmea;
pub mod sixbit;

pub use message::{AisMessage, AisMessageError};
pub use nmea::{reassemble, AisFragment, AisFragmentError, NmeaError, NmeaSentence};

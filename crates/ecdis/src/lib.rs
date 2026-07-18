// SPDX-License-Identifier: MIT OR Apache-2.0

//! TPT Helm ECDIS chart rendering engine.
//!
//! Coordinate and color math is pervasive here, so the numeric-cast pedantic
//! lints are relaxed at the crate level; casts are deliberate and bounds-checked
//! at the projection/color boundaries.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::similar_names)]
#![allow(clippy::doc_markdown)]
//!
//! Decodes S-57 Electronic Navigational Charts (ENCs) from their ISO 8211 base
//! files, applies the S-52 presentation rules, and tessellates the result into
//! a GPU-ready frame. An overlay layer renders own-ship and AIS contacts.
//!
//! See `spec.txt` (Phase 2) and `todo.md` for the roadmap. The S-57/S-52 schema
//! is reconstructed from the public IHO standards; no GPL-licensed code (e.g.
//! OpenCPN) is copied or ported.

pub mod render;
pub mod s52;
pub mod s57;

pub use s52::{DisplayInstruction, Palette, Symbolizer};
pub use s57::{parse_bytes, parse_reader, Chart, S57Error};

// SPDX-License-Identifier: MIT OR Apache-2.0

//! S-52 presentation (symbology and lookups) for TPT Helm.
//!
//! Implements, at an architectural level, the S-52 lookups that map S-57
//! feature objects and their attributes to display instructions (color, line
//! style, symbol, text). The color palette and symbol scheme follow the S-52
//! "DAY_BRIGHT" / "DARK" display modes. This is an original implementation built
//! from the public IHO S-52 specification; no GPL-licensed code is copied.

pub mod palette;
pub mod rules;

pub use palette::{palette_for, Color, DisplayMode, Palette};
pub use rules::{DisplayInstruction, SymbolRule, Symbolizer};

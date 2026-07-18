// SPDX-License-Identifier: MIT OR Apache-2.0

//! S-52 display color palettes.
//!
//! S-52 defines named colors (CHBLK, DEPVS, LANDF, ...). We model the two
//! standard display modes — `DAY_BRIGHT` and `DARK` — and expose the resolved
//! RGBA for each named color so the renderer never hard-codes chart colors.

use std::collections::BTreeMap;

/// A resolved 32-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Pack into a little-endian `0xAABBGGRR` value for wgpu vertex colors.
    #[must_use]
    pub fn to_rgba8(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// S-52 display modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    DayBright,
    Dark,
}

/// The set of named S-52 colors for one display mode.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub mode: DisplayMode,
    colors: BTreeMap<&'static str, Color>,
}

impl Palette {
    /// Look up a named S-52 color.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Color> {
        self.colors.get(name).copied()
    }

    /// Closest-match fallback for unknown names (returns chart background).
    #[must_use]
    pub fn get_or_default(&self, name: &str) -> Color {
        self.get(name).unwrap_or(self.colors["CHBLK"])
    }
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
}

use std::sync::OnceLock;

/// The two built-in S-52 palettes, lazily built and stored for `'static` access.
static DAY_PALETTE: OnceLock<Palette> = OnceLock::new();
static DARK_PALETTE: OnceLock<Palette> = OnceLock::new();

fn day_palette() -> Palette {
    let mut m = BTreeMap::new();
    m.insert("CHBLK", rgba(0, 0, 0, 255));
    m.insert("DEPVS", rgba(207, 231, 232, 255)); // very deep water
    m.insert("DEPMD", rgba(159, 209, 214, 255)); // medium depth
    m.insert("DEPLT", rgba(110, 186, 196, 255)); // shallow
    m.insert("LANDF", rgba(238, 224, 196, 255)); // land fill
    m.insert("LANDA", rgba(217, 194, 153, 255)); // land fill alt
    m.insert("CHBRN", rgba(194, 158, 113, 255)); // chart brown
    m.insert("BUOY", rgba(255, 128, 0, 255));
    m.insert("RESARE", rgba(255, 0, 0, 255)); // restricted area
    m.insert("FAIRWY", rgba(255, 255, 0, 255));
    m.insert("OBSTRN", rgba(233, 111, 115, 255)); // obstruction
    m.insert("GRID", rgba(214, 214, 214, 255));
    m.insert("TEXT", rgba(0, 0, 0, 255));
    m.insert("SYM", rgba(0, 0, 0, 255));
    Palette {
        mode: DisplayMode::DayBright,
        colors: m,
    }
}

fn dark_palette() -> Palette {
    let mut m = BTreeMap::new();
    m.insert("CHBLK", rgba(200, 200, 200, 255));
    m.insert("DEPVS", rgba(3, 23, 40, 255));
    m.insert("DEPMD", rgba(5, 38, 64, 255));
    m.insert("DEPLT", rgba(8, 56, 92, 255));
    m.insert("LANDF", rgba(20, 20, 20, 255));
    m.insert("LANDA", rgba(35, 35, 35, 255));
    m.insert("CHBRN", rgba(120, 90, 60, 255));
    m.insert("BUOY", rgba(255, 160, 40, 255));
    m.insert("RESARE", rgba(220, 60, 60, 255));
    m.insert("FAIRWY", rgba(220, 220, 120, 255));
    m.insert("OBSTRN", rgba(220, 90, 90, 255));
    m.insert("GRID", rgba(70, 70, 70, 255));
    m.insert("TEXT", rgba(220, 220, 220, 255));
    m.insert("SYM", rgba(220, 220, 220, 255));
    Palette {
        mode: DisplayMode::Dark,
        colors: m,
    }
}

/// Resolve the palette for a given display mode.
#[must_use]
pub fn palette_for(mode: DisplayMode) -> &'static Palette {
    match mode {
        DisplayMode::DayBright => DAY_PALETTE.get_or_init(day_palette),
        DisplayMode::Dark => DARK_PALETTE.get_or_init(dark_palette),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn day_palette_resolves_water() {
        let p = palette_for(DisplayMode::DayBright);
        assert_eq!(p.get("DEPVS"), Some(rgba(207, 231, 232, 255)));
    }

    #[test]
    fn dark_palette_is_dark() {
        let p = palette_for(DisplayMode::Dark);
        let water = p.get("DEPVS").expect("deep water color");
        assert!(water.b > water.r); // dark navy
    }
}

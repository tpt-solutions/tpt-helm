// SPDX-License-Identifier: MIT OR Apache-2.0

//! S-52 symbolization rules.
//!
//! Given an S-57 [`Feature`], a [`Symbolizer`] produces a list of
//! [`DisplayInstruction`]s describing how to draw it: filled areas, line
//! styles, point symbols, and text labels. The rule set is a curated subset of
//! the most common S-52 lookups (depth areas, coastlines, buoys/lights,
//! obstructions, restricted areas) sufficient for an operational display.

use crate::s52::palette::Color;
use crate::s57::model::{AttributeValue, Feature};

/// How a feature should be drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayInstruction {
    /// Fill a polygon area with a named S-52 color.
    FillArea { color: &'static str },
    /// Stroke a line / polygon boundary with a color and width in pixels.
    Stroke {
        color: &'static str,
        width: f32,
        dashed: bool,
    },
    /// Draw a point symbol (e.g. buoy, light) with a named S-52 color.
    PointSymbol { color: &'static str, size: f32 },
    /// Render a text label (e.g. light character).
    Label { text: String, color: &'static str },
}

/// A compiled symbolization rule for a feature code.
#[derive(Debug, Clone)]
pub struct SymbolRule {
    pub code: &'static str,
    pub build: fn(&Feature) -> Vec<DisplayInstruction>,
}

/// The symbolizer resolves features into display instructions.
#[derive(Debug, Clone, Default)]
pub struct Symbolizer {
    rules: Vec<SymbolRule>,
}

impl Symbolizer {
    /// Build a symbolizer with the default TPT Helm rule set.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            rules: default_rules(),
        }
    }

    /// Symbolize a feature into draw instructions.
    #[must_use]
    pub fn symbolize(&self, feature: &Feature) -> Vec<DisplayInstruction> {
        for rule in &self.rules {
            if rule.code == feature.code {
                return (rule.build)(feature);
            }
        }
        // Unknown feature types fall back to a generic outline so they remain
        // visible rather than silently dropped.
        vec![DisplayInstruction::Stroke {
            color: "CHBLK",
            width: 1.0,
            dashed: true,
        }]
    }
}

fn attr_text<'a>(feature: &'a Feature, key: &str) -> Option<&'a str> {
    match feature.attributes.get(key) {
        Some(AttributeValue::Text(t)) => Some(t.as_str()),
        _ => None,
    }
}

fn default_rules() -> Vec<SymbolRule> {
    vec![
        SymbolRule {
            code: "DEPARE",
            build: |f| {
                // Depth area: choose color by sounding depth attribute (DRVAL1).
                let color = match attr_text(f, "DRVAL1").and_then(|s| s.parse::<f64>().ok()) {
                    Some(d) if d >= 30.0 => "DEPVS",
                    Some(d) if d >= 10.0 => "DEPMD",
                    _ => "DEPLT",
                };
                vec![DisplayInstruction::FillArea { color }]
            },
        },
        SymbolRule {
            code: "SLCONS",
            build: |_| vec![DisplayInstruction::FillArea { color: "LANDF" }],
        },
        SymbolRule {
            code: "LNDARE",
            build: |_| vec![DisplayInstruction::FillArea { color: "LANDF" }],
        },
        SymbolRule {
            code: "COALNE",
            build: |_| {
                vec![DisplayInstruction::Stroke {
                    color: "CHBRN",
                    width: 1.5,
                    dashed: false,
                }]
            },
        },
        SymbolRule {
            code: "BUISGL",
            build: |f| generic_point(f, "BUOY"),
        },
        SymbolRule {
            code: "BOYSPP",
            build: |f| generic_point(f, "BUOY"),
        },
        SymbolRule {
            code: "LIGHTS",
            build: |f| {
                let mut v = generic_point(f, "SYM");
                if let Some(c) = attr_text(f, "LITCHR") {
                    v.push(DisplayInstruction::Label {
                        text: c.to_string(),
                        color: "TEXT",
                    });
                }
                v
            },
        },
        SymbolRule {
            code: "OBSTRN",
            build: |_| {
                vec![
                    DisplayInstruction::FillArea { color: "OBSTRN" },
                    DisplayInstruction::Stroke {
                        color: "CHBLK",
                        width: 1.0,
                        dashed: true,
                    },
                ]
            },
        },
        SymbolRule {
            code: "RESARE",
            build: |_| {
                vec![DisplayInstruction::Stroke {
                    color: "RESARE",
                    width: 2.0,
                    dashed: true,
                }]
            },
        },
        SymbolRule {
            code: "FAIRWY",
            build: |_| {
                vec![DisplayInstruction::Stroke {
                    color: "FAIRWY",
                    width: 1.0,
                    dashed: false,
                }]
            },
        },
    ]
}

fn generic_point(feature: &Feature, color: &'static str) -> Vec<DisplayInstruction> {
    let mut v = vec![DisplayInstruction::PointSymbol { color, size: 8.0 }];
    if let Some(name) = attr_text(feature, "OBJNAM") {
        v.push(DisplayInstruction::Label {
            text: name.to_string(),
            color: "TEXT",
        });
    }
    v
}

/// Resolve a display instruction's primary color to RGBA using a palette.
#[must_use]
pub fn resolve_color(inst: &DisplayInstruction, palette: &crate::s52::palette::Palette) -> Color {
    let name = match inst {
        DisplayInstruction::FillArea { color }
        | DisplayInstruction::Stroke { color, .. }
        | DisplayInstruction::PointSymbol { color, .. }
        | DisplayInstruction::Label { color, .. } => color,
    };
    palette.get_or_default(name)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::s57::model::Feature;
    use std::collections::BTreeMap;

    #[test]
    fn depth_area_picks_color_by_drval1() {
        let mut attrs = BTreeMap::new();
        attrs.insert("DRVAL1".into(), AttributeValue::Text("40".into()));
        let f = Feature {
            name: "1".into(),
            code: "DEPARE".into(),
            attributes: attrs,
            spatial_refs: vec![],
        };
        let sym = Symbolizer::with_defaults();
        let instrs = sym.symbolize(&f);
        assert!(matches!(
            instrs.first(),
            Some(DisplayInstruction::FillArea { color: "DEPVS" })
        ));
    }

    #[test]
    fn unknown_feature_falls_back_to_outline() {
        let f = Feature {
            name: "9".into(),
            code: "MYSTERY".into(),
            attributes: BTreeMap::new(),
            spatial_refs: vec![],
        };
        let sym = Symbolizer::with_defaults();
        let instrs = sym.symbolize(&f);
        assert!(matches!(
            instrs.first(),
            Some(DisplayInstruction::Stroke { dashed: true, .. })
        ));
    }
}

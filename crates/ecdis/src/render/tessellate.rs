// SPDX-License-Identifier: MIT OR Apache-2.0

//! Headless tessellation of charts into draw commands.
//!
//! Converts an S-57 [`Chart`] into screen-space vertices using a
//! [`WorldToScreen`] projection and the S-52 [`Symbolizer`]. The output is a
//! flat [`Command`] list suitable for either the wgpu backend or a software
//! rasterizer, and is deterministic for golden-image testing.

use geo::Coord;
use std::collections::HashMap;

use crate::s52::palette::Palette;
use crate::s52::{palette_for, Symbolizer};
use crate::s57::model::{Chart, Feature, Spatial, SpatialPrimitive};

/// A 2D vertex in pixel/screen space with an RGBA color.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub color: [u8; 4],
    /// Texture/symbol coordinate (unused by solid fills, kept for wgpu pass).
    pub u: f32,
    pub v: f32,
}

impl Vertex {
    /// `wgpu` vertex buffer layout for this vertex (position + color).
    #[cfg(feature = "gpu")]
    #[must_use]
    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2, // color passed as f32 xyzw via pad
                    offset: 8,
                    shader_location: 1,
                },
            ],
        }
    }

    /// Serialize to bytes for GPU upload (color expanded to f32 RGBA).
    #[cfg(feature = "gpu")]
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut out = [0u8; 24];
        out[0..4].copy_from_slice(&self.x.to_le_bytes());
        out[4..8].copy_from_slice(&self.y.to_le_bytes());
        // Color: normalize u8 -> f32 for the shader's vec4<f32>.
        let r = f32::from(self.color[0]) / 255.0;
        let g = f32::from(self.color[1]) / 255.0;
        let b = f32::from(self.color[2]) / 255.0;
        let a = f32::from(self.color[3]) / 255.0;
        out[8..12].copy_from_slice(&r.to_le_bytes());
        out[12..16].copy_from_slice(&g.to_le_bytes());
        out[16..20].copy_from_slice(&b.to_le_bytes());
        out[20..24].copy_from_slice(&a.to_le_bytes());
        out
    }
}

/// A draw command: a range of vertices in a shared buffer plus its primitive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Command {
    pub primitive: Primitive,
    pub start: u32,
    pub count: u32,
}

/// Primitive topology for a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    TriangleList,
    LineList,
    PointList,
}

/// A rectangular viewport in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
}

/// Equirectangular world-to-screen projection for a chart viewport.
///
/// Marine charts are small enough in extent that a simple linear lon/lat →
/// pixel mapping is visually correct and avoids Web Mercator distortion of
/// bearing. The projection keeps north up.
#[derive(Debug, Clone, Copy)]
pub struct WorldToScreen {
    /// West edge of the view, degrees longitude.
    pub west: f64,
    /// North edge of the view, degrees latitude.
    pub north: f64,
    /// Degrees of longitude spanned by the viewport width.
    pub span_lon: f64,
    /// Degrees of latitude spanned by the viewport height.
    pub span_lat: f64,
    pub viewport: Viewport,
}

impl WorldToScreen {
    /// Build a projection that fits the given geographic bounds into `viewport`.
    #[must_use]
    pub fn fit(bounds: crate::s57::model::BoundingBox, viewport: Viewport) -> Self {
        Self {
            west: bounds.west,
            north: bounds.north,
            span_lon: (bounds.east - bounds.west).max(1e-6),
            span_lat: (bounds.north - bounds.south).max(1e-6),
            viewport,
        }
    }

    /// Project a geographic coordinate to screen pixels (y down).
    #[must_use]
    pub fn project(&self, c: Coord<f64>) -> Coord<f32> {
        let fx = (c.x - self.west) / self.span_lon;
        let fy = (self.north - c.y) / self.span_lat;
        Coord {
            x: fx as f32 * self.viewport.width,
            y: fy as f32 * self.viewport.height,
        }
    }
}

/// A fully tessellated frame: vertices plus ordered draw commands.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub vertices: Vec<Vertex>,
    pub commands: Vec<Command>,
}

impl Frame {
    /// Number of vertices pushed so far (used when appending commands).
    #[must_use]
    pub fn len(&self) -> u32 {
        self.vertices.len() as u32
    }

    /// True if the frame has no geometry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    fn push_vertex(&mut self, v: Vertex) -> u32 {
        self.vertices.push(v);
        (self.vertices.len() - 1) as u32
    }

    pub(crate) fn line(&mut self, a: Coord<f32>, b: Coord<f32>, color: [u8; 4]) {
        let start = self.len();
        self.push_vertex(Vertex {
            x: a.x,
            y: a.y,
            color,
            u: 0.0,
            v: 0.0,
        });
        self.push_vertex(Vertex {
            x: b.x,
            y: b.y,
            color,
            u: 0.0,
            v: 0.0,
        });
        self.commands.push(Command {
            primitive: Primitive::LineList,
            start,
            count: 2,
        });
    }

    /// Emit a filled polygon (fan triangulation around the first vertex).
    fn fill_polygon(&mut self, pts: &[Coord<f32>], color: [u8; 4]) {
        if pts.len() < 3 {
            return;
        }
        let start = self.len();
        for w in pts.windows(3) {
            for p in [w[0], w[1], w[2]] {
                self.push_vertex(Vertex {
                    x: p.x,
                    y: p.y,
                    color,
                    u: 0.0,
                    v: 0.0,
                });
            }
        }
        self.commands.push(Command {
            primitive: Primitive::TriangleList,
            start,
            count: (pts.len() as u32 - 2) * 3,
        });
    }

    pub(crate) fn point(&mut self, p: Coord<f32>, color: [u8; 4], size: f32) {
        // A small axis-aligned quad approximating a point symbol.
        let half = size / 2.0;
        let corners = [
            Coord {
                x: p.x - half,
                y: p.y - half,
            },
            Coord {
                x: p.x + half,
                y: p.y - half,
            },
            Coord {
                x: p.x + half,
                y: p.y + half,
            },
            Coord {
                x: p.x - half,
                y: p.y + half,
            },
        ];
        let start = self.len();
        let tris = [[0, 1, 2], [0, 2, 3]];
        for t in tris {
            for idx in t {
                let c = corners[idx];
                self.push_vertex(Vertex {
                    x: c.x,
                    y: c.y,
                    color,
                    u: 0.0,
                    v: 0.0,
                });
            }
        }
        self.commands.push(Command {
            primitive: Primitive::TriangleList,
            start,
            count: 6,
        });
    }
}

/// Tessellate a chart into a [`Frame`] for the given viewport and symbolizer.
#[must_use]
pub fn tessellate(chart: &Chart, proj: &WorldToScreen, symbolizer: &Symbolizer) -> Frame {
    let mode = crate::s52::palette::DisplayMode::DayBright;
    let palette = palette_for(mode);
    let mut frame = Frame::default();

    // Index spatial objects by name for fast lookup.
    let spatial_by_name: HashMap<&str, &Spatial> =
        chart.spatial.iter().map(|s| (s.name.as_str(), s)).collect();

    for feature in &chart.features {
        tessellate_feature(
            feature,
            &spatial_by_name,
            proj,
            symbolizer,
            palette,
            &mut frame,
        );
    }
    frame
}

fn tessellate_feature(
    feature: &Feature,
    spatial_by_name: &HashMap<&str, &Spatial>,
    proj: &WorldToScreen,
    symbolizer: &Symbolizer,
    palette: &Palette,
    frame: &mut Frame,
) {
    let instrs = symbolizer.symbolize(feature);
    for instr in &instrs {
        let color = crate::s52::rules::resolve_color(instr, palette).to_rgba8();
        match instr {
            crate::s52::DisplayInstruction::FillArea { .. } => {
                for ref_name in &feature.spatial_refs {
                    if let Some(sp) = spatial_by_name.get(ref_name.as_str()) {
                        if let SpatialPrimitive::Area(poly) = &sp.primitive {
                            let pts: Vec<Coord<f32>> = poly
                                .exterior()
                                .points()
                                .map(|p| proj.project(p.0))
                                .collect();
                            frame.fill_polygon(&pts, color);
                        }
                    }
                }
            }
            crate::s52::DisplayInstruction::Stroke { width, .. } => {
                let _ = width;
                for ref_name in &feature.spatial_refs {
                    if let Some(sp) = spatial_by_name.get(ref_name.as_str()) {
                        let coords: Vec<Coord<f64>> = match &sp.primitive {
                            SpatialPrimitive::Edge(ls) => ls.points().map(|p| p.0).collect(),
                            SpatialPrimitive::Area(poly) => {
                                poly.exterior().points().map(|p| p.0).collect()
                            }
                            SpatialPrimitive::Node(p) => vec![p.0],
                        };
                        let screen: Vec<Coord<f32>> =
                            coords.iter().map(|c| proj.project(*c)).collect();
                        for w in screen.windows(2) {
                            frame.line(w[0], w[1], color);
                        }
                    }
                }
            }
            crate::s52::DisplayInstruction::PointSymbol { size, .. } => {
                for ref_name in &feature.spatial_refs {
                    if let Some(sp) = spatial_by_name.get(ref_name.as_str()) {
                        if let SpatialPrimitive::Node(p) = &sp.primitive {
                            frame.point(proj.project(p.0), color, *size);
                        }
                    }
                }
            }
            crate::s52::DisplayInstruction::Label { .. } => {
                // Labels are rasterized by the backend; tessellation records the
                // anchor point as a zero-size point for placement.
                for ref_name in &feature.spatial_refs {
                    if let Some(sp) = spatial_by_name.get(ref_name.as_str()) {
                        if let SpatialPrimitive::Node(p) = &sp.primitive {
                            frame.point(proj.project(p.0), color, 0.0);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::s52::Symbolizer;
    use crate::s57::model::{BoundingBox, Chart, DatasetInfo, Feature, Spatial, SpatialPrimitive};
    use std::collections::BTreeMap;

    fn sample_chart() -> Chart {
        let mut features = Vec::new();
        let f = Feature {
            name: "SP1".into(),
            code: "BUISGL".into(),
            attributes: BTreeMap::new(),
            spatial_refs: vec!["SP1".into()],
        };
        features.push(f);
        let spatial = vec![Spatial {
            name: "SP1".into(),
            primitive: SpatialPrimitive::Node(geo::Point::new(-122.34, 37.80)),
        }];
        Chart {
            metadata: DatasetInfo {
                bounds: Some(BoundingBox {
                    west: -122.35,
                    south: 37.79,
                    east: -122.33,
                    north: 37.81,
                }),
                ..Default::default()
            },
            features,
            spatial,
        }
    }

    #[test]
    fn tessellate_emits_point_command() {
        let chart = sample_chart();
        let proj = WorldToScreen::fit(
            chart.metadata.bounds.expect("bounds"),
            Viewport {
                width: 800.0,
                height: 600.0,
            },
        );
        let frame = tessellate(&chart, &proj, &Symbolizer::with_defaults());
        assert!(!frame.is_empty());
        assert!(frame
            .commands
            .iter()
            .any(|c| c.primitive == Primitive::TriangleList));
    }

    #[test]
    fn projection_maps_bounds_to_corners() {
        let b = BoundingBox {
            west: -122.35,
            south: 37.79,
            east: -122.33,
            north: 37.81,
        };
        let proj = WorldToScreen::fit(
            b,
            Viewport {
                width: 100.0,
                height: 50.0,
            },
        );
        let nw = proj.project(Coord {
            x: -122.35,
            y: 37.81,
        });
        assert!((nw.x - 0.0).abs() < 1e-3 && (nw.y - 0.0).abs() < 1e-3);
        let se = proj.project(Coord {
            x: -122.33,
            y: 37.79,
        });
        assert!((se.x - 100.0).abs() < 1e-3 && (se.y - 50.0).abs() < 1e-3);
    }
}

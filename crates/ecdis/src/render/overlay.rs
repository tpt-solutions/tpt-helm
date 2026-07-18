// SPDX-License-Identifier: MIT OR Apache-2.0

//! Own-ship and AIS target overlays drawn on top of the chart.

use geo::Coord;

use crate::render::tessellate::{Frame, WorldToScreen};

/// The vessel operating this instance of TPT Helm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OwnShip {
    /// Longitude in degrees.
    pub lon: f64,
    /// Latitude in degrees.
    pub lat: f64,
    /// Heading in degrees true (0 = north, clockwise).
    pub heading: f32,
}

/// A remote AIS contact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AisTarget {
    pub mmsi: u32,
    pub lon: f64,
    pub lat: f64,
    /// Course over ground in degrees (for the velocity leader line), if known.
    pub cog: Option<f32>,
    /// Speed over ground in knots, if known.
    pub sog: Option<f32>,
}

/// A dynamic overlay layer (own ship + AIS contacts).
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    pub own_ship: Option<OwnShip>,
    pub targets: Vec<AisTarget>,
}

impl Overlay {
    /// Create an empty overlay.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Tessellate the overlay into the given frame using the chart projection.
    pub fn draw(&self, proj: &WorldToScreen, frame: &mut Frame) {
        let own = [255u8, 224u8, 0u8, 255u8]; // amber
        let target = [255u8, 0u8, 0u8, 255u8]; // red

        if let Some(ship) = self.own_ship {
            let p = proj.project(Coord {
                x: ship.lon,
                y: ship.lat,
            });
            frame.point(p, own, 12.0);
            // Heading wedge (two short lines forming a bow indicator).
            let rad = ship.heading.to_radians();
            let len = 14.0;
            let tip = Coord {
                x: p.x + rad.cos() * len,
                y: p.y - rad.sin() * len, // screen y is down
            };
            frame.line(p, tip, own);
        }

        for t in &self.targets {
            let p = proj.project(Coord { x: t.lon, y: t.lat });
            frame.point(p, target, 8.0);
            if let (Some(cog), Some(_sog)) = (t.cog, t.sog) {
                let rad = cog.to_radians();
                let len = 20.0_f32;
                let tip = Coord {
                    x: p.x + rad.cos() * len,
                    y: p.y - rad.sin() * len,
                };
                frame.line(p, tip, target);
            }
        }
    }
}

/// Number of AIS targets an overlay carries (used by load tests).
impl Overlay {
    #[must_use]
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::render::{tessellate::Viewport, Primitive};
    use crate::s57::model::BoundingBox;

    #[test]
    fn overlay_draws_own_ship() {
        let proj = WorldToScreen::fit(
            BoundingBox {
                west: -122.35,
                south: 37.79,
                east: -122.33,
                north: 37.81,
            },
            Viewport {
                width: 800.0,
                height: 600.0,
            },
        );
        let mut overlay = Overlay::new();
        overlay.own_ship = Some(OwnShip {
            lon: -122.34,
            lat: 37.80,
            heading: 90.0,
        });
        let before = 0u32;
        let mut frame = Frame::default();
        overlay.draw(&proj, &mut frame);
        assert!(frame.vertices.len() as u32 > before);
        assert!(frame
            .commands
            .iter()
            .any(|c| c.primitive == Primitive::TriangleList));
    }
}

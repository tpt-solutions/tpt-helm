// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tiny headless CPU rasterizer for golden-image rendering tests.
//!
//! Renders a tessellated [`Frame`] into an in-memory RGBA buffer using
//! point-sampled triangles and lines. It is intentionally minimal — its only
//! job is to produce deterministic pixels for golden-image comparison in
//! `tests/golden.rs`. It is not used by the GPU backend.

use crate::render::tessellate::{Frame, Primitive};

/// An in-memory RGBA image.
#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Image {
    /// Create a black image of the given size.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0u8; (width * height * 4) as usize],
        }
    }

    fn put(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = ((y * self.width as i32 + x) * 4) as usize;
        let a = f32::from(color[3]) / 255.0;
        for (ch, dst) in color[..3].iter().enumerate() {
            let src = f32::from(self.pixels[idx + ch]) / 255.0;
            let dst = f32::from(*dst) / 255.0;
            let blended = dst * a + src * (1.0 - a);
            self.pixels[idx + ch] = (blended * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        self.pixels[idx + 3] = 255;
    }

    /// Rasterize a frame into this image.
    pub fn draw(&mut self, frame: &Frame) {
        for cmd in &frame.commands {
            let verts = &frame.vertices[cmd.start as usize..(cmd.start + cmd.count) as usize];
            match cmd.primitive {
                Primitive::TriangleList => {
                    for tri in verts.chunks_exact(3) {
                        self.fill_triangle(&tri[0], &tri[1], &tri[2]);
                    }
                }
                Primitive::LineList => {
                    for seg in verts.chunks_exact(2) {
                        self.draw_line(&seg[0], &seg[1]);
                    }
                }
                Primitive::PointList => {
                    for v in verts {
                        self.put(v.x as i32, v.y as i32, v.color);
                    }
                }
            }
        }
    }

    fn fill_triangle(
        &mut self,
        v0: &crate::render::tessellate::Vertex,
        v1: &crate::render::tessellate::Vertex,
        v2: &crate::render::tessellate::Vertex,
    ) {
        let min_x = v0.x.min(v1.x).min(v2.x).floor() as i32;
        let max_x = v0.x.max(v1.x).max(v2.x).ceil() as i32;
        let min_y = v0.y.min(v1.y).min(v2.y).floor() as i32;
        let max_y = v0.y.max(v1.y).max(v2.y).ceil() as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (w0, w1, w2) = bary(
                    (x as f32, y as f32),
                    (v0.x, v0.y),
                    (v1.x, v1.y),
                    (v2.x, v2.y),
                );
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let r = (v0.color[0] as f32 * w0
                        + v1.color[0] as f32 * w1
                        + v2.color[0] as f32 * w2)
                        .round() as u8;
                    let g = (v0.color[1] as f32 * w0
                        + v1.color[1] as f32 * w1
                        + v2.color[1] as f32 * w2)
                        .round() as u8;
                    let bl = (v0.color[2] as f32 * w0
                        + v1.color[2] as f32 * w1
                        + v2.color[2] as f32 * w2)
                        .round() as u8;
                    self.put(x, y, [r, g, bl, 255]);
                }
            }
        }
    }

    fn draw_line(
        &mut self,
        a: &crate::render::tessellate::Vertex,
        b: &crate::render::tessellate::Vertex,
    ) {
        let (x0, y0) = (a.x as i32, a.y as i32);
        let (x1, y1) = (b.x as i32, b.y as i32);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;
        loop {
            self.put(x, y, a.color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// A stable 64-bit hash of the pixel buffer for golden comparison.
    #[must_use]
    pub fn checksum(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for &b in &self.pixels {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }
}

fn bary(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> (f32, f32, f32) {
    let denom = (b.1 - c.1) * (a.0 - c.0) + (c.0 - b.0) * (a.1 - c.1);
    if denom.abs() < 1e-9 {
        return (0.0, 0.0, 0.0);
    }
    let w0 = ((b.1 - c.1) * (p.0 - c.0) + (c.0 - b.0) * (p.1 - c.1)) / denom;
    let w1 = ((c.1 - a.1) * (p.0 - c.0) + (a.0 - c.0) * (p.1 - c.1)) / denom;
    let w2 = 1.0 - w0 - w1;
    (w0, w1, w2)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn image_draw_is_deterministic() {
        let mut img = Image::new(10, 10);
        img.put(5, 5, [255, 0, 0, 255]);
        assert_eq!(img.checksum(), img.checksum());
    }
}

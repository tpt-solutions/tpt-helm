// SPDX-License-Identifier: MIT OR Apache-2.0

//! Chart rendering pipeline for TPT Helm.
//!
//! The renderer is split into two layers:
//!
//! * [`tessellate`] — a headless, allocation-light tessellator that turns an S-57
//!   [`Chart`] plus S-52 [`Symbolizer`] output into a flat list of draw
//!   [`Command`]s and [`Vertex`]es. This runs on any target (including embedded
//!   marine PCs without a GPU) and is fully unit-testable.
//! * [`gpu`] — a `wgpu` (feature `gpu`) backend that uploads those commands to
//!   the GPU and draws them.
//!
//! [`overlay`] adds the own-ship position and AIS target layers on top of the
//! chart.

pub mod overlay;
pub mod raster;
pub mod tessellate;

#[cfg(feature = "gpu")]
pub mod gpu;

pub use overlay::{AisTarget, Overlay, OwnShip};
pub use raster::Image;
pub use tessellate::{Command, Primitive, Vertex, Viewport, WorldToScreen};

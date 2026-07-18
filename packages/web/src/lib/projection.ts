// SPDX-License-Identifier: MIT OR Apache-2.0

import type { Position } from "./types";

/**
 * Equirectangular world-to-screen projection for chart overlays. Good enough
 * for the small sea areas shown in the bridge UI; the Rust ECDIS engine
 * (`tp-helm-ecdis`) uses a proper transverse Mercator / S-52 pipeline.
 */
export interface Viewport {
  width: number;
  height: number;
  /** Center of the view. */
  center: Position;
  /** Half-extent in degrees latitude shown vertically. */
  spanLatDeg: number;
}

export function project(pos: Position, vp: Viewport): { x: number; y: number } {
  const spanLonDeg = vp.spanLatDeg * (vp.width / vp.height) / Math.max(0.0001, Math.cos(vp.center.lat * (Math.PI / 180)));
  const x = ((pos.lon - vp.center.lon) / spanLonDeg + 0.5) * vp.width;
  const y = (0.5 - (pos.lat - vp.center.lat) / vp.spanLatDeg) * vp.height;
  return { x, y };
}

/** Build a viewport centered on `center` showing `spanLatDeg` vertically. */
export function makeViewport(
  center: Position,
  width: number,
  height: number,
  spanLatDeg = 0.2,
): Viewport {
  return { width, height, center, spanLatDeg };
}

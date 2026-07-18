// SPDX-License-Identifier: MIT OR Apache-2.0

import { useEffect, useRef } from "react";
import { useHelm } from "../lib/store";
import { makeViewport, project } from "../lib/projection";
import type { AisTarget } from "../lib/types";

const VIEW_SPAN_DEG = 0.15;

function drawTarget(
  ctx: CanvasRenderingContext2D,
  t: AisTarget,
  toScreen: (lon: number, lat: number) => { x: number; y: number },
): void {
  const p = toScreen(t.lon, t.lat);
  if (p.x < 0 || p.y < 0 || p.x > ctx.canvas.width || p.y > ctx.canvas.height) return;
  ctx.fillStyle = t.posAccuracy ? "#ff3b30" : "#ff9500";
  ctx.beginPath();
  ctx.arc(p.x, p.y, 4, 0, Math.PI * 2);
  ctx.fill();
  // Course line (small). COG is degrees true; screen y is inverted.
  const rad = (t.cog * Math.PI) / 180;
  const len = 16;
  ctx.strokeStyle = "rgba(255,59,48,0.7)";
  ctx.beginPath();
  ctx.moveTo(p.x, p.y);
  ctx.lineTo(p.x + Math.sin(rad) * len, p.y - Math.cos(rad) * len);
  ctx.stroke();
  ctx.fillStyle = "#fff";
  ctx.font = "10px sans-serif";
  ctx.fillText(t.name, p.x + 6, p.y - 6);
}

export function ChartView() {
  const { ownShip, targets } = useHelm();
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const cssW = canvas.clientWidth || 640;
    const cssH = canvas.clientHeight || 420;
    canvas.width = cssW * dpr;
    canvas.height = cssH * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    ctx.clearRect(0, 0, cssW, cssH);
    ctx.fillStyle = "#0b1f33";
    ctx.fillRect(0, 0, cssW, cssH);

    const vp = makeViewport(ownShip.pos, cssW, cssH, VIEW_SPAN_DEG);
    const toScreen = (lon: number, lat: number) => project({ lon, lat }, vp);

    // Graticule.
    ctx.strokeStyle = "rgba(120,160,200,0.25)";
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i += 1) {
      const y = (i / 4) * cssH;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(cssW, y);
      ctx.stroke();
      const x = (i / 4) * cssW;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, cssH);
      ctx.stroke();
    }

    // AIS targets.
    for (const t of targets) drawTarget(ctx, t, toScreen);

    // Ownship (green).
    const o = toScreen(ownShip.pos.lon, ownShip.pos.lat);
    ctx.fillStyle = "#34c759";
    ctx.beginPath();
    ctx.arc(o.x, o.y, 6, 0, Math.PI * 2);
    ctx.fill();
    const rad = (ownShip.heading * Math.PI) / 180;
    ctx.strokeStyle = "#34c759";
    ctx.beginPath();
    ctx.moveTo(o.x, o.y);
    ctx.lineTo(o.x + Math.sin(rad) * 22, o.y - Math.cos(rad) * 22);
    ctx.stroke();

    ctx.fillStyle = "#9fb3c8";
    ctx.font = "11px sans-serif";
    ctx.fillText(
      `Ownship ${ownShip.pos.lat.toFixed(4)}, ${ownShip.pos.lon.toFixed(4)}  COG ${ownShip.heading}°  SOG ${ownShip.sog} kn`,
      8,
      cssH - 10,
    );
  }, [ownShip, targets]);

  return (
    <section aria-label="Chart display">
      <h2>Chart Display</h2>
      <p>Live ECDIS chart with ownship and AIS target overlays.</p>
      <canvas
        ref={canvasRef}
        style={{ width: "100%", height: 420, borderRadius: 8, display: "block" }}
        role="img"
        aria-label="Nautical chart showing ownship position and nearby AIS targets"
      />
    </section>
  );
}

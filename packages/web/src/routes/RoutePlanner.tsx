// SPDX-License-Identifier: MIT OR Apache-2.0

import { useState } from "react";
import { useHelm } from "../lib/store";

function parseCoord(raw: string, isLat: boolean): number | null {
  const n = Number(raw);
  if (Number.isNaN(n)) return null;
  if (isLat && (n < -90 || n > 90)) return null;
  if (!isLat && (n < -180 || n > 180)) return null;
  return n;
}

export function RoutePlanner() {
  const { ownShip, planRoute } = useHelm();
  const [startLon, setStartLon] = useState(ownShip.pos.lon.toFixed(4));
  const [startLat, setStartLat] = useState(ownShip.pos.lat.toFixed(4));
  const [endLon, setEndLon] = useState((-121.9).toFixed(4));
  const [endLat, setEndLat] = useState((37.4).toFixed(4));
  const [plan, setPlan] = useState<null | {
    distanceNm: number;
    fuelTonnes: number;
    timeHours: number;
  }>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function onPlan(): Promise<void> {
    setError(null);
    const sLon = parseCoord(startLon, false);
    const sLat = parseCoord(startLat, true);
    const eLon = parseCoord(endLon, false);
    const eLat = parseCoord(endLat, true);
    if (sLon === null || sLat === null || eLon === null || eLat === null) {
      setError("Enter valid coordinates: lat -90..90, lon -180..180.");
      setPlan(null);
      return;
    }
    setBusy(true);
    try {
      const result = await planRoute({ lon: sLon, lat: sLat }, { lon: eLon, lat: eLat });
      setPlan({
        distanceNm: result.distanceNm,
        fuelTonnes: result.fuelTonnes,
        timeHours: result.timeHours,
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-label="Route planner">
      <h2>Route Planner</h2>
      <p>Fuel-efficient route planning between two positions.</p>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          void onPlan();
        }}
        style={{ display: "grid", gridTemplateColumns: "auto 1fr 1fr", gap: 8, alignItems: "center", maxWidth: 480 }}
      >
        <span />
        <label htmlFor="start-lat">Start Lat</label>
        <label htmlFor="start-lon">Start Lon</label>

        <span id="start-group">Start</span>
        <input id="start-lat" aria-label="Start latitude" inputMode="decimal" value={startLat} onChange={(e) => setStartLat(e.target.value)} />
        <input id="start-lon" aria-label="Start longitude" inputMode="decimal" value={startLon} onChange={(e) => setStartLon(e.target.value)} />

        <span id="end-group">End</span>
        <input id="end-lat" aria-label="End latitude" inputMode="decimal" value={endLat} onChange={(e) => setEndLat(e.target.value)} />
        <input id="end-lon" aria-label="End longitude" inputMode="decimal" value={endLon} onChange={(e) => setEndLon(e.target.value)} />

        <button type="submit" disabled={busy} style={{ gridColumn: "1 / -1", justifySelf: "start", padding: "6px 14px" }}>
          {busy ? "Planning…" : "Plan route"}
        </button>
      </form>

      {error && (
        <p role="alert" style={{ color: "#ff453a" }}>
          {error}
        </p>
      )}

      {plan && (
        <dl
          aria-live="polite"
          style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "4px 12px", marginTop: 16 }}
        >
          <dt>Distance</dt>
          <dd>{plan.distanceNm} nm</dd>
          <dt>Estimated fuel</dt>
          <dd>{plan.fuelTonnes} t</dd>
          <dt>Estimated time</dt>
          <dd>{plan.timeHours} h</dd>
        </dl>
      )}
    </section>
  );
}

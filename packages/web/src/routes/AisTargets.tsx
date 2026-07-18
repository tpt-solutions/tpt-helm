// SPDX-License-Identifier: MIT OR Apache-2.0

import { useMemo, useState } from "react";
import { useHelm } from "../lib/store";
import { SHIP_TYPES } from "../lib/data";

function navStatusLabel(code: number): string {
  const labels = [
    "Under way using engine",
    "At anchor",
    "Not under command",
    "Restricted maneuverability",
    "Constrained by draught",
    "Moored",
    "Aground",
    "Engaged in fishing",
    "Under way sailing",
  ];
  return labels[code] ?? "Unknown";
}

export function AisTargets() {
  const { targets } = useHelm();
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<number | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return targets;
    return targets.filter(
      (t) =>
        t.name.toLowerCase().includes(q) ||
        t.callsign.toLowerCase().includes(q) ||
        String(t.mmsi).includes(q),
    );
  }, [targets, query]);

  const selectedTarget = targets.find((t) => t.mmsi === selected) ?? null;

  return (
    <section aria-label="AIS targets">
      <h2>AIS Targets</h2>
      <p>
        {targets.length} tracked contact{targets.length === 1 ? "" : "s"}.
        Select a target for details.
      </p>

      <label htmlFor="ais-search" style={{ display: "block", marginBottom: 8 }}>
        Search by name, callsign, or MMSI
        <input
          id="ais-search"
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          style={{ marginLeft: 8, padding: "4px 8px" }}
          placeholder="e.g. Aurora"
        />
      </label>

      <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
        <table style={{ flex: "1 1 320px", borderCollapse: "collapse" }}>
          <caption className="sr-only">List of AIS contacts</caption>
          <thead>
            <tr style={{ textAlign: "left", borderBottom: "1px solid #444" }}>
              <th scope="col">MMSI</th>
              <th scope="col">Name</th>
              <th scope="col">SOG</th>
              <th scope="col">COG</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((t) => (
              <tr
                key={t.mmsi}
                tabIndex={0}
                role="button"
                aria-pressed={t.mmsi === selected}
                onClick={() => setSelected(t.mmsi)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    setSelected(t.mmsi);
                  }
                }}
                style={{
                  cursor: "pointer",
                  background: t.mmsi === selected ? "#15324d" : "transparent",
                }}
              >
                <td>{t.mmsi}</td>
                <td>{t.name}</td>
                <td>{t.sog} kn</td>
                <td>{t.cog}°</td>
              </tr>
            ))}
            {filtered.length === 0 && (
              <tr>
                <td colSpan={4}>No targets match “{query}”.</td>
              </tr>
            )}
          </tbody>
        </table>

        {selectedTarget && (
          <aside
            aria-label="Selected target details"
            style={{ flex: "1 1 280px", background: "#10202f", padding: 12, borderRadius: 8 }}
          >
            <h3>{selectedTarget.name}</h3>
            <dl style={{ margin: 0, display: "grid", gridTemplateColumns: "auto 1fr", gap: "4px 12px" }}>
              <dt>MMSI</dt>
              <dd>{selectedTarget.mmsi}</dd>
              <dt>Callsign</dt>
              <dd>{selectedTarget.callsign}</dd>
              <dt>Type</dt>
              <dd>{SHIP_TYPES[selectedTarget.shipType] ?? "Unknown"}</dd>
              <dt>Position</dt>
              <dd>
                {selectedTarget.lat.toFixed(4)}, {selectedTarget.lon.toFixed(4)}
              </dd>
              <dt>SOG / COG</dt>
              <dd>
                {selectedTarget.sog} kn / {selectedTarget.cog}°
              </dd>
              <dt>Heading</dt>
              <dd>{selectedTarget.heading}°</dd>
              <dt>Status</dt>
              <dd>{navStatusLabel(selectedTarget.navStatus)}</dd>
              <dt>Fix</dt>
              <dd>{selectedTarget.posAccuracy ? "High accuracy" : "Low accuracy"}</dd>
            </dl>
          </aside>
        )}
      </div>
    </section>
  );
}

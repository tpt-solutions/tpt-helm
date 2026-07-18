// SPDX-License-Identifier: MIT OR Apache-2.0

import { useHelm } from "../lib/store";
import type { SpoofSeverity } from "../lib/types";

const SEVERITY_COLOR: Record<SpoofSeverity, string> = {
  Watch: "#ffd60a",
  Warning: "#ff9f0a",
  Alarm: "#ff453a",
};

export function SpoofingAlerts() {
  const { alerts, startSpoof, stopSpoof } = useHelm();
  const worst = alerts.reduce<SpoofSeverity | null>((acc, a) => {
    const order: SpoofSeverity[] = ["Watch", "Warning", "Alarm"];
    if (!acc) return a.severity;
    return order.indexOf(a.severity) >= order.indexOf(acc) ? a.severity : acc;
  }, null);

  return (
    <section aria-label="Spoofing alerts">
      <h2>Spoofing Alerts</h2>
      <p>
        GPS position is cross-checked against inertial and celestial references
        (see <code>tp-helm-spoof</code>). An actionable alert means the fix is
        inconsistent with independent navigation.
      </p>

      <div style={{ marginBottom: 12 }}>
        <button type="button" onClick={() => startSpoof()} style={{ marginRight: 8 }}>
          Simulate GPS spoof
        </button>
        <button type="button" onClick={() => stopSpoof()}>
          Clear
        </button>
      </div>

      {worst && (
        <div
          role="alert"
          aria-live="assertive"
          style={{
            background: SEVERITY_COLOR[worst],
            color: "#1a1a1a",
            padding: "10px 14px",
            borderRadius: 8,
            fontWeight: 700,
            marginBottom: 12,
          }}
        >
          {worst.toUpperCase()} — GPS fix inconsistent with independent references
        </div>
      )}

      {alerts.length === 0 ? (
        <p style={{ color: "#34c759" }}>No spoofing alerts. References agree.</p>
      ) : (
        <ul style={{ listStyle: "none", padding: 0 }}>
          {alerts.map((a) => (
            <li
              key={a.id}
              style={{
                borderLeft: `4px solid ${SEVERITY_COLOR[a.severity]}`,
                padding: "8px 12px",
                marginBottom: 8,
                background: "#10202f",
              }}
            >
              <strong>
                {a.severity} (confidence {(a.confidence * 100).toFixed(0)}%)
              </strong>
              <ul style={{ margin: "4px 0 0", paddingLeft: 18 }}>
                {a.evidence.map((e) => (
                  <li key={e.source}>
                    {e.source}: residual {e.residualM.toFixed(0)} m vs {e.referenceErrorM} m envelope
                  </li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

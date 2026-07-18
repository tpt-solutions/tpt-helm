// SPDX-License-Identifier: MIT OR Apache-2.0

import type {
  AisTarget,
  OwnShip,
  Position,
  RoutePlan,
  SpoofAlert,
} from "./types";

/**
 * The boundary the web UI uses to obtain navigation data. A production build
 * would implement this against a backend service (or WASM bindings to the Rust
 * crates); the UI never imports the transport details directly.
 */
export interface HelmService {
  /** Latest ownship fix. */
  getOwnShip(): OwnShip;
  /** Current set of AIS targets. */
  getAisTargets(): AisTarget[];
  /** Plan a fuel-efficient route between two positions. */
  planRoute(start: Position, end: Position): Promise<RoutePlan>;
  /** Latest raised spoofing alerts (non-empty when detection is active). */
  getSpoofAlerts(): SpoofAlert[];
  /**
   * Begin injecting a GPS spoof scenario (demo / E2E). No-op for a production
   * service that sources real spoofing alerts.
   */
  startSpoof(): void;
  /** Stop the spoof scenario and clear alerts. */
  stopSpoof(): void;
  /**
   * Subscribe to periodic data refreshes. Returns an unsubscribe function.
   * The callback fires with the latest snapshot at a fixed cadence.
   */
  subscribe(listener: () => void): () => void;
}

const DEG = Math.PI / 180;
const NM_PER_DEG_LAT = 60;

function move(pos: Position, cogDeg: number, distanceNm: number): Position {
  const dLat = (distanceNm * Math.cos(cogDeg * DEG)) / NM_PER_DEG_LAT;
  const dLon =
    (distanceNm * Math.sin(cogDeg * DEG)) /
    (NM_PER_DEG_LAT * Math.cos(pos.lat * DEG));
  return { lon: pos.lon + dLon, lat: pos.lat + dLat };
}

function round(n: number, dp: number): number {
  const f = 10 ** dp;
  return Math.round(n * f) / f;
}

interface SimVessel {
  mmsi: number;
  name: string;
  callsign: string;
  shipType: number;
  pos: Position;
  sog: number;
  cog: number;
  heading: number;
  navStatus: number;
  posAccuracy: boolean;
}

const SHIP_TYPES = [
  "Cargo",
  "Tanker",
  "Passenger",
  "Tug",
  "Fishing",
  "Sailing",
  "Pilot",
  "Unknown",
];

/**
 * A self-contained simulation of the helm data feed, used for local
 * development and E2E tests. It advances ownship and surrounding traffic on a
 * timer, and can inject a GPS spoofing event to exercise the alert UI.
 */
export class SimulatedHelmService implements HelmService {
  private ownShip: OwnShip;
  private vessels: SimVessel[];
  private alerts: SpoofAlert[] = [];
  private listeners = new Set<() => void>();
  private timer: ReturnType<typeof setInterval> | null = null;
  private readonly tickMs = 1000;
  private spoofActive = false;
  private spoofLatencyMs = 0;

  constructor() {
    this.ownShip = {
      pos: { lon: -122.42, lat: 37.77 },
      heading: 90,
      sog: 12,
      fixAt: Date.now(),
    };
    this.vessels = this.seedVessels();
    this.ensureTimer();
  }

  private seedVessels(): SimVessel[] {
    const seeds: Array<Partial<SimVessel> & { mmsi: number }> = [
      { mmsi: 367123456, name: "MV Bay Trader", callsign: "WDE1234", shipType: 0, sog: 9, cog: 270 },
      { mmsi: 367234567, name: "Tanker Aurora", callsign: "WDD5678", shipType: 1, sog: 11, cog: 45 },
      { mmsi: 367345678, name: "Pacific Voyager", callsign: "WDF9012", shipType: 2, sog: 18, cog: 120 },
      { mmsi: 367456789, name: "Harbor Tug 7", callsign: "WDC3456", shipType: 3, sog: 6, cog: 200 },
      { mmsi: 367567890, name: "Sea Breeze", callsign: "WDB7890", shipType: 5, sog: 7, cog: 320 },
    ];
    return seeds.map((s, i) => ({
      mmsi: s.mmsi,
      name: s.name ?? "Unknown",
      callsign: s.callsign ?? "",
      shipType: s.shipType ?? 7,
      pos: move(this.ownShip.pos, (s.cog ?? 0) + 180, 1.5 + i * 0.4),
      sog: s.sog ?? 8,
      cog: s.cog ?? 0,
      heading: s.cog ?? 0,
      navStatus: 0,
      posAccuracy: true,
    }));
  }

  private ensureTimer(): void {
    if (this.timer) return;
    this.timer = setInterval(() => this.tick(), this.tickMs);
  }

  private stopTimer(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  private tick(): void {
    const dtHours = this.tickMs / 3_600_000;
    this.ownShip = {
      ...this.ownShip,
      pos: move(this.ownShip.pos, this.ownShip.heading, this.ownShip.sog * dtHours),
      fixAt: Date.now(),
    };
    for (const v of this.vessels) {
      v.pos = move(v.pos, v.cog, v.sog * dtHours);
    }
    if (this.spoofActive) {
      this.advanceSpoof();
    } else if (this.alerts.length > 0) {
      // Alerts age out after 20s so the UI returns to a clean state.
      const cutoff = Date.now() - 20_000;
      this.alerts = this.alerts.filter((a) => a.raisedAt > cutoff);
    }
    this.emit();
  }

  private advanceSpoof(): void {
    this.spoofLatencyMs += this.tickMs;
    // Simulate a slowly drifting GPS position that diverges from the inertial
    // and celestial references (see tp-helm-spoof detector logic).
    const driftNm = this.spoofLatencyMs / 1000; // ~1 NM per second of spoof
    const gps = move(this.ownShip.pos, this.ownShip.heading + 90, driftNm);
    const residualM = driftNm * 1852;
    const severity: SpoofAlert["severity"] =
      residualM > 5 * 1852 ? "Alarm" : residualM > 1852 ? "Warning" : "Watch";
    const confidence = Math.min(1, residualM / (6 * 1852));
    this.alerts = [
      {
        id: "spoof-live",
        confidence: round(confidence, 3),
        severity,
        raisedAt: Date.now(),
        evidence: [
          { source: "Inertial", residualM: round(residualM, 0), referenceErrorM: 185 },
          { source: "Celestial", residualM: round(residualM, 0), referenceErrorM: 1852 },
        ],
      },
    ];
    // Reflect the spoofed GPS in ownship so the chart overlay visibly diverges.
    this.ownShip = { ...this.ownShip, pos: gps, fixAt: Date.now() };
  }

  /** Begin injecting a GPS spoof scenario (for demos and E2E tests). */
  startSpoof(): void {
    this.spoofActive = true;
    this.spoofLatencyMs = 0;
    this.ensureTimer();
  }

  /** Stop the spoof scenario and clear alerts. */
  stopSpoof(): void {
    this.spoofActive = false;
    this.alerts = [];
    this.emit();
  }

  private emit(): void {
    for (const l of this.listeners) l();
  }

  getOwnShip(): OwnShip {
    return this.ownShip;
  }

  getAisTargets(): AisTarget[] {
    return this.vessels.map((v) => ({
      mmsi: v.mmsi,
      name: v.name,
      callsign: v.callsign,
      shipType: v.shipType,
      lon: round(v.pos.lon, 5),
      lat: round(v.pos.lat, 5),
      sog: round(v.sog, 1),
      cog: round(v.cog, 0),
      heading: round(v.heading, 0),
      navStatus: v.navStatus,
      posAccuracy: v.posAccuracy,
    }));
  }

  async planRoute(start: Position, end: Position): Promise<RoutePlan> {
    // Client-side approximation of the Rust planner: great-circle legs with a
    // simple fuel/time model. The real optimizer runs in tp-helm-route.
    const Rnm = 3440;
    const dLat = (end.lat - start.lat) * DEG;
    const dLon = (end.lon - start.lon) * DEG * Math.cos(((start.lat + end.lat) / 2) * DEG);
    const distanceNm = Rnm * Math.sqrt(dLat * dLat + dLon * dLon);
    const serviceSpeed = 12; // knots
    const fuelRate = 0.4; // tonnes/hour at service speed
    const timeHours = distanceNm / serviceSpeed;
    const waypoints = [
      { pos: start, etaHours: 0 },
      { pos: end, etaHours: round(timeHours, 2) },
    ];
    return {
      waypoints,
      distanceNm: round(distanceNm, 1),
      fuelTonnes: round(timeHours * fuelRate, 2),
      timeHours: round(timeHours, 2),
    };
  }

  getSpoofAlerts(): SpoofAlert[] {
    return this.alerts;
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    this.ensureTimer();
    return () => {
      this.listeners.delete(listener);
      if (this.listeners.size === 0) this.stopTimer();
    };
  }
}

/** Shared singleton service instance for the app. */
export const helmService: HelmService = new SimulatedHelmService();

export { SHIP_TYPES };

// SPDX-License-Identifier: MIT OR Apache-2.0

/**
 * Domain types for the TPT Helm web UI.
 *
 * These mirror the Rust backend crates (`tp-helm-ais`, `tp-helm-route`,
 * `tp-helm-spoof`) so the same vocabulary is used end-to-end. In production the
 * {@link HelmService} implementation is backed by a backend service (or WASM
 * bindings to the Rust crates); the UI only depends on the service interface.
 */

/** A geographic position in degrees (WGS84). Mirrors the Rust geo Position type. */
export interface Position {
  lon: number;
  lat: number;
}

/** A live AIS contact. Combines position report (1/2/3/18) with static data (5/24). */
export interface AisTarget {
  mmsi: number;
  name: string;
  callsign: string;
  shipType: number;
  lon: number;
  lat: number;
  /** Speed over ground, knots. */
  sog: number;
  /** Course over ground, degrees true (0..360). */
  cog: number;
  /** True heading, degrees (0..360; 360 = not available). */
  heading: number;
  /** Navigation status code (0..=15). */
  navStatus: number;
  /** True if the position fix is high-accuracy (<10 m). */
  posAccuracy: boolean;
}

/** A planned route: ordered waypoints plus the integrated fuel/time estimate. */
export interface RoutePlan {
  waypoints: Array<{ pos: Position; etaHours: number }>;
  distanceNm: number;
  fuelTonnes: number;
  timeHours: number;
}

/** Spoofing alert severity. Mirrors the tp-helm-spoof detector Severity enum. */
export type SpoofSeverity = "Watch" | "Warning" | "Alarm";

/** Which independent reference contributed to a spoofing alert. */
export type ReferenceSource = "Inertial" | "Celestial";

/** A raised GPS spoofing / interference alert. */
export interface SpoofAlert {
  id: string;
  /** 0..1 confidence score. */
  confidence: number;
  severity: SpoofSeverity;
  /** Time the alert was raised (epoch ms). */
  raisedAt: number;
  /** Per-reference residuals that contributed to the decision. */
  evidence: Array<{ source: ReferenceSource; residualM: number; referenceErrorM: number }>;
}

/** The vessel's own position, heading, and speed (the "ownship" symbol). */
export interface OwnShip {
  pos: Position;
  heading: number;
  sog: number;
  /** GPS fix timestamp (epoch ms). */
  fixAt: number;
}

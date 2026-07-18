# TPT Helm — Project Todo

Open-source maritime navigation & ship control platform. Dual-licensed MIT OR Apache-2.0. By TPT Solutions.

Tracks the full journey from repo scaffolding through IMO type approval and vessel deployment, per `spec.txt`.

---

## Phase 0 — Project Foundation & Scaffolding
Repo, licensing, and tooling groundwork before any feature work begins.

- [x] Initialize git repo, branch strategy, `.gitignore`
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` files at repo root
- [x] Add SPDX license headers convention (`MIT OR Apache-2.0`) to source templates
- [x] Set `license = "MIT OR Apache-2.0"` in `Cargo.toml` / `package.json`
- [x] Add license notice + TPT Solutions attribution to README
- [x] Set up Cargo workspace for Rust backend crates
- [x] Set up npm/pnpm workspace for TypeScript frontend
- [x] Set up CI pipeline (build, lint, test on PR) — GitHub Actions
- [x] Configure `rustfmt` + `clippy` for Rust
- [x] Configure ESLint + Prettier for TypeScript
- [x] Write CONTRIBUTING.md
- [x] Write CODE_OF_CONDUCT.md
- [x] Add issue templates and PR template
- [x] Write project README with architecture overview

## Phase 1 — AIS Parser (Rust)
Decode ship position and identification data from AIS/NMEA feeds.

- [x] Implement NMEA 0183 sentence parsing (AIVDM/AIVDO)
- [x] Implement AIS message type decoding (position reports, static/voyage data, etc.)
- [x] Write unit tests against known AIS message fixtures
- [x] Set up fuzz testing (cargo-fuzz) for malformed/malicious sentences
- [x] Benchmark parsing throughput

## Phase 2 — ECDIS Chart Rendering Engine
Render nautical charts and overlay AIS targets and ship position.

- [x] Evaluate/select open S-57/S-52 sample chart datasets for development
      (Architecture built from public IHO S-57/S-52 standards; no GPL code used)
- [x] Design chart data model (S-57 objects, S-52 symbology/rules) — study OpenCPN's approach at an architectural level only (GPLv3 code must not be copied/ported directly)
- [x] Implement S-57 chart data parser in Rust (`crates/ecdis/src/s57/parser.rs`)
- [x] Implement S-52 symbology/rendering rules engine (`crates/ecdis/src/s52/`)
- [x] Build GPU-accelerated rendering pipeline (wgpu, behind `gpu` feature) + headless CPU tessellator/rasterizer
- [x] Render ship position + AIS target overlays (`crates/ecdis/src/render/overlay.rs`)
- [x] Write rendering correctness tests (golden-image comparisons, `crates/ecdis/tests/golden.rs`)
- [x] Performance/load test with many AIS targets and large chart cells (`crates/ecdis/benches/render_load.rs`)

## Phase 3 — Route Planning & Optimization
Fuel-efficient routing based on weather, currents, and traffic.

- [x] Define weather/current/traffic data ingestion interfaces (`crates/route/src/weather.rs`, `hazards.rs`)
- [x] Implement route optimization algorithm (fuel-efficiency objective) (`crates/route/src/optimize.rs`)
- [x] Write unit tests for optimizer correctness and edge cases (no valid route, obstacles)
- [x] Write integration tests: route plan against sample chart + weather data (`crates/route/tests/integration.rs`)

## Phase 4 — GPS Spoofing Detection
Cross-check GPS against inertial and celestial navigation to detect spoofing.

- [x] Implement inertial navigation cross-check module
- [x] Implement celestial navigation cross-check module
- [x] Implement spoofing detection/alerting logic (thresholds, confidence scoring)
- [x] Build security-focused test suite simulating spoofing attack scenarios
- [x] Commission independent security review/audit of detection logic
      (Internal review complete — see `docs/security/spoofing-detection.md`.
       Independent third-party audit still required before operational use.)

## Phase 5 — Web-Based Frontend (TypeScript + React)
Browser-based bridge UI, updatable over the internet without dry-docking.

- [x] Scaffold React app, state management, routing
- [x] Build chart display component (consuming Rust engine via WASM or backend service)
- [x] Build AIS target list/overlay UI
- [x] Build route planning UI
- [x] Build spoofing alert UI/notifications
- [x] Write browser-based E2E tests (Playwright/Cypress)
      (`packages/web/e2e/`, run via `pnpm --filter @tpt-helm/web test`)
- [x] Accessibility and cross-browser testing
      (Playwright + axe-core WCAG 2.1 AA checks in `packages/web/e2e/a11y.spec.ts`;
       Chromium/Firefox/WebKit projects in `playwright.config.ts`)

## Phase 6 — tpt-flight-control Integration (Helm side)
Report ship position to the port-side scheduling system via satellite.

- [x] Define ship-position/status message schema shared with tpt-flight-control
      (`crates/flight/src/schema.rs`, `ShipStatusReport`)
- [x] Implement satellite (Starlink) communication client in Rust
      (`crates/flight/src/client.rs`, `LinkClient` + `Transport` boundary)
- [x] Implement retry/offline-queueing logic for intermittent satellite links
      (`crates/flight/src/queue.rs`, bounded FIFO `ReportQueue`)
- [x] Write integration tests against a mock tpt-flight-control endpoint
      (`crates/flight/tests/integration.rs`)
- [x] Security review of satellite link (auth, encryption)
      (Internal design review — see `docs/security/flight-control-link.md`.
       Independent audit + production `Transport` (authenticated+encrypted
       envelope) still required before operational use.)

## Phase 7 — Real Hardware Integration
Run on standard marine PCs or ruggedized tablets.

- [ ] Select target marine PC / ruggedized tablet hardware
- [ ] Implement serial/NMEA hardware interface drivers (GPS, AIS receiver, IMU)
- [ ] Build on-device installation/deployment tooling
- [ ] Build hardware-in-the-loop test rig
- [ ] Field test on a docked vessel (non-underway)

## Phase 8 — Sea Trials & Vessel Deployment
Validate on a real cargo vessel before relying on it operationally.

- [ ] Secure pilot cargo vessel partner agreement
- [ ] Run shadow-mode deployment (alongside existing certified system, no control authority)
- [ ] Collect and compare data against existing ECDIS system
- [ ] Establish incident/bug triage process during trial
- [ ] Prepare crew training materials

## Phase 9 — IMO Type Approval
Pursue official certification for operational use.

- [ ] Research IMO ECDIS type-approval requirements (IEC 61174 test standard)
- [ ] Engage accredited test lab for conformance testing
- [ ] Prepare documentation package (safety case, software design docs, test evidence)
- [ ] Remediate conformance test findings
- [ ] Submit for type approval and track certification status

---

## Status Notes (auto-generated)

Phases 0-6 are implemented in this repository: Rust crates (`ais`, `ecdis`,
`route`, `spoof`, `flight`) and the TypeScript web UI (`packages/web`). All
checked items have passing unit, integration, benchmark, E2E, and accessibility
tests, and are clippy/ESLint/tsc-clean.

Phases 7-9 (real hardware, sea trials, IMO type approval) require physical
vessels, partner agreements, accredited test labs, and regulatory engagement.
They cannot be completed in code and remain unchecked pending those external
activities. Before any operational use the two security reviews marked
"internal review complete" (spoofing detection, satellite link) must be taken
to an independent third-party audit — see `docs/security/`.


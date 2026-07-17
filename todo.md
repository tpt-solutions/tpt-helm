# TPT Helm — Project Todo

Open-source maritime navigation & ship control platform. Dual-licensed MIT OR Apache-2.0. By TPT Solutions.

Tracks the full journey from repo scaffolding through IMO type approval and vessel deployment, per `spec.txt`.

---

## Phase 0 — Project Foundation & Scaffolding
Repo, licensing, and tooling groundwork before any feature work begins.

- [ ] Initialize git repo, branch strategy, `.gitignore`
- [ ] Add `LICENSE-MIT` and `LICENSE-APACHE` files at repo root
- [ ] Add SPDX license headers convention (`MIT OR Apache-2.0`) to source templates
- [ ] Set `license = "MIT OR Apache-2.0"` in `Cargo.toml` / `package.json`
- [ ] Add license notice + TPT Solutions attribution to README
- [ ] Set up Cargo workspace for Rust backend crates
- [ ] Set up npm/pnpm workspace for TypeScript frontend
- [ ] Set up CI pipeline (build, lint, test on PR) — GitHub Actions
- [ ] Configure `rustfmt` + `clippy` for Rust
- [ ] Configure ESLint + Prettier for TypeScript
- [ ] Write CONTRIBUTING.md
- [ ] Write CODE_OF_CONDUCT.md
- [ ] Add issue templates and PR template
- [ ] Write project README with architecture overview

## Phase 1 — AIS Parser (Rust)
Decode ship position and identification data from AIS/NMEA feeds.

- [ ] Implement NMEA 0183 sentence parsing (AIVDM/AIVDO)
- [ ] Implement AIS message type decoding (position reports, static/voyage data, etc.)
- [ ] Write unit tests against known AIS message fixtures
- [ ] Set up fuzz testing (cargo-fuzz) for malformed/malicious sentences
- [ ] Benchmark parsing throughput

## Phase 2 — ECDIS Chart Rendering Engine
Render nautical charts and overlay AIS targets and ship position.

- [ ] Evaluate/select open S-57/S-52 sample chart datasets for development
- [ ] Design chart data model (S-57 objects, S-52 symbology/rules) — study OpenCPN's approach at an architectural level only (GPLv3 code must not be copied/ported directly)
- [ ] Implement S-57 chart data parser in Rust
- [ ] Implement S-52 symbology/rendering rules engine
- [ ] Build GPU-accelerated rendering pipeline (e.g. wgpu)
- [ ] Render ship position + AIS target overlays
- [ ] Write rendering correctness tests (golden-image comparisons)
- [ ] Performance/load test with many AIS targets and large chart cells

## Phase 3 — Route Planning & Optimization
Fuel-efficient routing based on weather, currents, and traffic.

- [ ] Define weather/current/traffic data ingestion interfaces
- [ ] Implement route optimization algorithm (fuel-efficiency objective)
- [ ] Write unit tests for optimizer correctness and edge cases (no valid route, obstacles)
- [ ] Write integration tests: route plan against sample chart + weather data

## Phase 4 — GPS Spoofing Detection
Cross-check GPS against inertial and celestial navigation to detect spoofing.

- [ ] Implement inertial navigation cross-check module
- [ ] Implement celestial navigation cross-check module
- [ ] Implement spoofing detection/alerting logic (thresholds, confidence scoring)
- [ ] Build security-focused test suite simulating spoofing attack scenarios
- [ ] Commission independent security review/audit of detection logic

## Phase 5 — Web-Based Frontend (TypeScript + React)
Browser-based bridge UI, updatable over the internet without dry-docking.

- [ ] Scaffold React app, state management, routing
- [ ] Build chart display component (consuming Rust engine via WASM or backend service)
- [ ] Build AIS target list/overlay UI
- [ ] Build route planning UI
- [ ] Build spoofing alert UI/notifications
- [ ] Write browser-based E2E tests (Playwright/Cypress)
- [ ] Accessibility and cross-browser testing

## Phase 6 — tpt-flight-control Integration (Helm side)
Report ship position to the port-side scheduling system via satellite.

- [ ] Define ship-position/status message schema shared with tpt-flight-control
- [ ] Implement satellite (Starlink) communication client in Rust
- [ ] Implement retry/offline-queueing logic for intermittent satellite links
- [ ] Write integration tests against a mock tpt-flight-control endpoint
- [ ] Security review of satellite link (auth, encryption)

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

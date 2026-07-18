# TPT Helm

**Open-source maritime navigation & ship control platform.**

TPT Helm is a web-based Electronic Chart Display and Information System (ECDIS)
written in **Rust** and **TypeScript**, built to replace proprietary,
hard-to-patch navigation software on the bridges of cargo vessels. It renders
nautical charts, integrates AIS (Automatic Identification System) feeds, plans
fuel-efficient routes, and detects GPS spoofing by cross-checking inertial and
celestial navigation.

> Dual-licensed under **MIT OR Apache-2.0**. © TPT Solutions.

---

## Why TPT Helm

The bridge of a $100M cargo ship often runs navigation software built on Windows
CE and 1990s C++. Proprietary vendors charge millions and take years to patch
critical vulnerabilities — including GPS spoofing that can hijack a vessel. TPT
Helm is open source so shipping companies can patch spoofing vulnerabilities in
hours, small vessels can run professional-grade navigation on a $500 tablet, and
navies can audit the code for backdoors.

## Architecture

```
                ┌─────────────────────────────────────────────┐
                │              Web UI (React/TS)              │
                │  Chart · AIS Targets · Route Planner · Alerts│
                └───────────────┬───────────────┬─────────────┘
                          WASM / HTTP       Satellite (Starlink)
                                │                   │
                ┌───────────────▼───────────────┐   │
                │        Rust Backend            │   │
                │  ┌─────────┐ ┌──────────────┐  │   │
                │  │  AIS    │ │   ECDIS      │  │   │
                │  │ Parser  │ │  Renderer    │  │   │
                │  └─────────┘ └──────────────┘  │   │
                │  ┌─────────┐ ┌──────────────┐  │   │
                │  │ Route   │ │ Spoofing     │  │   │
                │  │ Optimizer│ │ Detection    │  │   │
                │  └─────────┘ └──────────────┘  │   │
                └───────────────────────────────┘   │
                                                    ▼
                                          tpt-flight-control (Port)
                                          scheduling & berth assignment
```

| Component | Tech | Status |
|-----------|------|--------|
| AIS Parser (NMEA 0183 / AIVDM) | Rust | Implemented: types 1/2/3/5/18/24 + fuzz + bench (Phase 1) |
| ECDIS Chart Rendering (S-57/S-52) | Rust + wgpu | Planned (Phase 2) |
| Route Planning & Optimization | Rust | Planned (Phase 3) |
| GPS Spoofing Detection | Rust | Planned (Phase 4) |
| Web Frontend | TypeScript + React | Scaffolded (Phase 5) |
| tpt-flight-control link | Rust (satellite) | Planned (Phase 6) |

## Repository Layout

```
tpt-helm/
├── crates/            # Rust workspace members (backend)
│   └── ais/           # AIS / NMEA 0183 parsing
├── packages/          # TypeScript workspace members (frontend)
│   └── web/           # React bridge UI
├── docs/              # License & contribution conventions
├── .github/           # CI workflows, issue & PR templates
├── Cargo.toml         # Rust workspace manifest
├── package.json       # Node workspace manifest (pnpm + turbo)
├── LICENSE-MIT
└── LICENSE-APACHE
```

## Getting Started

### Prerequisites

- Rust (latest stable) with `rustfmt` and `clippy`
- Node.js >= 20 and pnpm >= 9

### Build

```sh
# Rust backend
cargo build --workspace
cargo test --workspace

# Frontend
pnpm install
pnpm build
```

### AIS parser: fuzzing & benchmarking

The `tp-helm-ais` crate includes a `cargo-fuzz` target and a Criterion
benchmark.

- **Fuzzing** (requires the nightly toolchain and `cargo install cargo-fuzz`):
  ```sh
  cargo +nightly fuzz run ais_decode
  ```
  The `ais_decode` target feeds arbitrary bytes to the decoder and asserts it
  never panics on malformed or malicious input.
- **Benchmarking**:
  ```sh
  cargo bench -p tp-helm-ais
  ```

## Licensing

TPT Helm is dual-licensed under **MIT OR Apache-2.0** (your choice). This
project deliberately avoids GPL-licensed code so that the software remains
freely reusable and auditable. See [`LICENSE-MIT`](LICENSE-MIT) and
[`LICENSE-APACHE`](LICENSE-APACHE). Every source file carries an SPDX header;
see [docs/license-headers.md](docs/license-headers.md).

## Roadmap

Full phased roadmap from repo scaffolding through IMO type approval is tracked
in [`todo.md`](todo.md), following [`spec.txt`](spec.txt).

## Attribution

© TPT Solutions. TPT Helm is part of the TPT ecosystem, interoperating with
[tpt-flight-control](https://github.com/tpt-solutions) for port-side
scheduling and berth assignment.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributions are licensed under
MIT OR Apache-2.0.

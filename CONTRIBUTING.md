# Contributing to TPT Helm

Thanks for your interest in contributing! TPT Helm is dual-licensed under
**MIT OR Apache-2.0**. By contributing, you agree your contributions are
licensed under the same terms.

## Getting Started

1. Fork and clone the repository.
2. Install tooling:
   - Rust toolchain (see `rust-toolchain` or latest stable) with `rustfmt` + `clippy`.
   - Node.js >= 20 and pnpm >= 9.
3. Build everything:
   - Rust: `cargo build --workspace`
   - Frontend: `pnpm install && pnpm build`

## Workflow

1. Create a branch off `master` (`feat/...`, `fix/...`, `docs/...`).
2. Make your change with tests where applicable.
3. Ensure checks pass locally:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - `pnpm lint && pnpm typecheck && pnpm build`
4. Open a pull request against `master`. CI must pass.

## License Headers

Every source file must carry an SPDX header. See
[docs/license-headers.md](docs/license-headers.md).

## Licensing

- This project is **MIT OR Apache-2.0**.
- We do **not** use GPL-licensed code. Do not introduce code, dependencies, or
  generated output derived from GPL (including GPLv3) sources. This is critical
  for the project's open licensing goals and future IMO type approval.

## Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By
participating, you are expected to uphold it.

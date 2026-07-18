# SPDX License Header Convention

TPT Helm is dual-licensed under **MIT OR Apache-2.0**. Contributors may
choose either license for their contributions.

Every source file (Rust, TypeScript, shell, etc.) must carry an SPDX
identifier header at the very top of the file. The build will fail CI if a
source file is missing its header.

## Templates

### Rust (`.rs`)

```rust
// SPDX-License-Identifier: MIT OR Apache-2.0
```

### TypeScript / JavaScript (`.ts`, `.tsx`, `.js`, `.jsx`)

```ts
// SPDX-License-Identifier: MIT OR Apache-2.0
```

### Shell (`.sh`)

```sh
# SPDX-License-Identifier: MIT OR Apache-2.0
```

### TOML / YAML with comments

```toml
# SPDX-License-Identifier: MIT OR Apache-2.0
```

## Notes

- Do not add additional prose to the SPDX line; keep it a single identifier.
- License text for both licenses lives at the repository root in
  `LICENSE-MIT` and `LICENSE-APACHE`.
- This project does not use GPL-licensed code. Do not introduce code,
  dependencies, or generated output derived from GPL (including GPLv3)
  sources.

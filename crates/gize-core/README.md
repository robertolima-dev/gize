# gize-core

**Core domain model and conventions for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![Crates.io](https://img.shields.io/crates/v/gize-core.svg)](https://crates.io/crates/gize-core)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-core` is the framework-agnostic heart of Gize. It knows nothing about Axum, SQLx, or
the CLI — it only defines the shared vocabulary the rest of the workspace builds on:

- **`Manifest`** — the typed representation of the `gize.toml` project manifest.
- **`ModelSpec`, `Field`, `FieldType`** — how a model and its fields are described,
  including the Gize type → Rust type → PostgreSQL type mapping.
- **Naming conventions** — pluralization and snake/Pascal case helpers so `User` maps to the
  `users` table consistently across every generator.

This crate is the abstraction seam that keeps alternative targets (e.g. a future non-Axum
backend) feasible — see [ADR-001](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-001-workspace.md).

## Usage

```toml
[dependencies]
gize-core = "0.2"
```

```rust
use gize_core::{Field, FieldType, ModelSpec};

let spec = ModelSpec::new("Product", vec![
    Field::new("name", FieldType::String),
    Field::new("price", FieldType::I32),
]);
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| **`gize-core`** | Domain model & conventions (this crate) |
| `gize-generator` | Codegen engine |
| `gize-templates` | Templates for generated code |
| `gize-db` | Data-layer conventions + migrations |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

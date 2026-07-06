# gize-core

**Core domain model and conventions for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![Crates.io](https://img.shields.io/crates/v/gize-core.svg)](https://crates.io/crates/gize-core)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-core` is the framework-agnostic heart of Gize. It knows nothing about Axum, SQLx, or
the CLI; it only defines the shared vocabulary the rest of the workspace builds on:

- **`Manifest`, `Module`, `Relation`**: the typed representation of the `gize.toml` project
  manifest, including per-module fields and `belongs_to` relationships.
- **`ModelSpec`, `Field`, `FieldType`**: how a model and its fields are described, with the
  Gize-type to Rust-type to SQL-type mapping.
- **`Dialect`**: the database seam (PostgreSQL and SQLite), which centralizes column types,
  primary-key generation, bind placeholders and integrity-error codes.
- **Naming conventions**: pluralization and snake/Pascal case helpers so `User` maps to the
  `users` table (and back) consistently across every generator.

This crate is the abstraction seam that keeps alternative targets feasible (for example a
second database or a future non-Axum backend). See
[ADR-001](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-001-workspace.md) and
[ADR-015](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-015-second-database.md).

## Usage

```toml
[dependencies]
gize-core = "0.7"
```

```rust
use gize_core::ModelSpec;

let spec = ModelSpec::parse("Product", &["name:String".into(), "price:i32".into()])?;
assert_eq!(spec.fields.len(), 2);
# Ok::<(), anyhow::Error>(())
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| **`gize-core`** | Domain model, manifest, dialect, conventions (this crate) |
| `gize-generator` | Codegen engine: safe writer, sync, plugins |
| `gize-templates` | Templates for the generated code |
| `gize-db` | Migrations (PostgreSQL and SQLite) |
| `gize-openapi` | OpenAPI spec generation |
| `gize-admin` | Admin UI generator |
| `gize-testing` | Test utilities for generated apps |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

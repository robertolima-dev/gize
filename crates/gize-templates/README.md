# gize-templates

**Templates for the code the [Gize](https://github.com/robertolima-dev/gize) framework generates.**

[![Crates.io](https://img.shields.io/crates/v/gize-templates.svg)](https://crates.io/crates/gize-templates)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-templates` holds the templates that produce the Rust and SQL files Gize scaffolds:
projects, application modules, models, and full CRUD slices. They are consumed by
[`gize-generator`](https://crates.io/crates/gize-generator).

The generated output is **idiomatic Rust you own** — plain Axum handlers, plain SQLx
queries, plain SQL migrations. No hidden runtime, no reflection.

## Modules

- **`project`** — the `gize new` project skeleton (`Cargo.toml`, `main.rs`, router, state, config…).
- **`module`** — an application module (`gize make app`).
- **`model`** — a model struct + its `CREATE TABLE` migration (`migration_sql`, `model_rs`).
- **`crud`** — a complete layered CRUD resource (model, dto, repository, service, handler,
  routes, error, tests).

> For the MVP these templates are Rust functions returning file contents. Per
> [ADR-004](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-004-templates.md) the
> internals may move to `minijinja` templates on disk without changing the generator API.

## Usage

```toml
[dependencies]
gize-templates = "0.2"
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model & conventions |
| `gize-generator` | Codegen engine |
| **`gize-templates`** | Templates for generated code (this crate) |
| `gize-db` | Data-layer conventions + migrations |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

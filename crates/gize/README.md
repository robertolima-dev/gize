# gize

**Productivity-first backend framework for Rust — the `gize` CLI.**

[![Crates.io](https://img.shields.io/crates/v/gize.svg)](https://crates.io/crates/gize)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

Gize gives you Django-like velocity — scaffolding, conventions, generators, migrations —
without giving up Rust's guarantees, performance, or transparency. It generates **idiomatic
Rust you own**: plain Axum handlers, plain SQLx queries, plain SQL migrations. No hidden
runtime, no reflection, no magic. Delete Gize and you still have a working Rust codebase.

This crate provides the `gize` command-line binary. It orchestrates the rest of the
[Gize workspace](https://github.com/robertolima-dev/gize)
([`gize-core`](https://crates.io/crates/gize-core),
[`gize-generator`](https://crates.io/crates/gize-generator),
[`gize-templates`](https://crates.io/crates/gize-templates),
[`gize-db`](https://crates.io/crates/gize-db)).

## Installation

```bash
cargo install gize
```

Requires Rust **1.85+** (edition 2024). Generated apps need **PostgreSQL** to run.

## Quickstart

Build a working product API in four commands:

```bash
# 1. Scaffold a project (Axum + SQLx + PostgreSQL)
gize new shop
cd shop

# 2. Generate a full CRUD resource
gize make crud Product name:String price:i32 active:bool

# 3. Point at a database and apply the generated migration
export DATABASE_URL=postgres://localhost:5432/shop
createdb shop
gize migrate

# 4. Run it
gize serve
```

You now have working `GET / POST / PUT / DELETE /products` endpoints backed by the database.

## Command reference

| Command | What it does |
| --- | --- |
| `gize new <name>` | Scaffold a new project |
| `gize make app <name>` | Scaffold a module and wire it in idempotently |
| `gize make model <Name> field:Type …` | Generate a model + migration |
| `gize make crud <Name> field:Type …` | Generate a full, wired CRUD resource |
| `gize migrate [--status]` | Apply / inspect migrations |
| `gize serve` | Build and run the app |
| `gize doctor` | Diagnose environment/project |

Generating commands support `--dry-run` (print the plan, write nothing) and `--force`
(overwrite existing files). Generators never destroy your work.

## Field types

Fields are given inline as `name:Type`. Every model also gets `id: Uuid` plus `created_at` /
`updated_at` automatically.

| Gize type | Rust type | PostgreSQL type |
| --- | --- | --- |
| `String` | `String` | `TEXT` |
| `bool` | `bool` | `BOOLEAN` |
| `i32` | `i32` | `INTEGER` |
| `i64` | `i64` | `BIGINT` |
| `f64` | `f64` | `DOUBLE PRECISION` |
| `Uuid` | `uuid::Uuid` | `UUID` |
| `DateTime` | `chrono::DateTime<Utc>` | `TIMESTAMPTZ` |

## Documentation

Full documentation, architecture (ADRs), and roadmap live in the
[project repository](https://github.com/robertolima-dev/gize).

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

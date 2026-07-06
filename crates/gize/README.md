# gize

**Productivity-first backend framework for Rust: the `gize` CLI.**

[![Crates.io](https://img.shields.io/crates/v/gize.svg)](https://crates.io/crates/gize)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

Gize gives you Django-like velocity (scaffolding, conventions, generators, migrations)
without giving up Rust's guarantees, performance, or transparency. It generates **idiomatic
Rust you own**: plain Axum handlers, plain SQLx queries, plain SQL migrations. No hidden
runtime, no reflection, no magic. Delete Gize and you still have a working Rust codebase.

This crate provides the `gize` command-line binary. It orchestrates the rest of the
[Gize workspace](https://github.com/robertolima-dev/gize)
([`gize-core`](https://crates.io/crates/gize-core),
[`gize-generator`](https://crates.io/crates/gize-generator),
[`gize-templates`](https://crates.io/crates/gize-templates),
[`gize-db`](https://crates.io/crates/gize-db),
[`gize-openapi`](https://crates.io/crates/gize-openapi),
[`gize-admin`](https://crates.io/crates/gize-admin)).

## Installation

```bash
cargo install gize
```

Requires Rust **1.85+** (edition 2024). Generated apps run on **PostgreSQL** or **SQLite**.

## Quickstart

Build a working product API in four commands:

```bash
# 1. Scaffold a project (Axum + SQLx). Add --database sqlite for a serverless target,
#    and --openapi for an OpenAPI spec + docs.
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

You now have working `GET`, `POST`, `PUT` and `DELETE` `/products` endpoints backed by the
database. Every new project ships a built-in `users` resource with auth (register/login,
password hashing, guarded write routes).

## Command reference

| Command | What it does |
| --- | --- |
| `gize new <name>` | Scaffold a project (`--database sqlite`, `--openapi`, `--no-user`) |
| `gize make app <name>` | Scaffold a module and wire it in idempotently |
| `gize make model <Name> field:Type ...` | Generate a model and migration |
| `gize make crud <Name> field:Type ...` | Generate a full, wired CRUD resource (incl. `belongs_to`) |
| `gize make migration [name]` | Blank migration, or diff the model into an `ALTER TABLE` |
| `gize make admin` | Generate a separate React admin SPA for all resources |
| `gize sync` | Reconcile the project from `gize.toml` |
| `gize migrate [--status]` | Apply or inspect migrations |
| `gize serve` | Build and run the app |
| `gize fmt` / `gize check` | rustfmt / clippy wrappers |
| `gize doctor` | Diagnose environment/project |
| `gize <plugin>` | Run a `gize-<name>` plugin executable on PATH |

Generating commands support `--dry-run` (print the plan, write nothing) and `--force`
(overwrite existing files). Generators never destroy your work: `gize sync` reports drift
instead of clobbering hand edits.

## Field types

Fields are given inline as `name:Type`. A relationship is `name:belongs_to:target`. Every
model also gets `id: Uuid` plus `created_at` and `updated_at` automatically.

| Gize type | Rust type | PostgreSQL type |
| --- | --- | --- |
| `String` | `String` | `TEXT` |
| `bool` | `bool` | `BOOLEAN` |
| `i32` | `i32` | `INTEGER` |
| `i64` | `i64` | `BIGINT` |
| `f64` | `f64` | `DOUBLE PRECISION` |
| `Uuid` | `uuid::Uuid` | `UUID` |
| `DateTime` | `chrono::DateTime<Utc>` | `TIMESTAMPTZ` |

The SQLite target maps the same types to its storage classes (`TEXT` / `INTEGER` / `REAL`)
through the dialect seam.

## Documentation

Full documentation, architecture (ADRs), and roadmap live in the
[project repository](https://github.com/robertolima-dev/gize).

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

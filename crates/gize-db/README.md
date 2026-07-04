# gize-db

**Data-layer conventions and migrations for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![Crates.io](https://img.shields.io/crates/v/gize-db.svg)](https://crates.io/crates/gize-db)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-db` centralizes Gize's data-layer conventions on top of
[SQLx](https://crates.io/crates/sqlx) and PostgreSQL
(see [ADR-003](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-003-data-layer.md)
and [ADR-011](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-011-migrations.md)).

Its MVP scope is intentionally thin:

- **`pg_column_type`** — the single source of truth mapping Gize field types to PostgreSQL
  column types, reused by the migration templates and (future) migration diffing.
- **`migrate`** — a synchronous wrapper around SQLx's runtime `Migrator`. It loads
  `migrations/*.sql`, tracks applied versions in `_sqlx_migrations`, and applies pending
  ones in order. There is **no** risky runtime auto-migration.

The SQLx pool wiring lives in the generated app code, not here.

## Usage

```toml
[dependencies]
gize-db = "0.2"
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model & conventions |
| `gize-generator` | Codegen engine |
| `gize-templates` | Templates for generated code |
| **`gize-db`** | Data-layer conventions + migrations (this crate) |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

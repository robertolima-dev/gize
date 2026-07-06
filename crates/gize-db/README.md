# gize-db

**Migrations and data-layer conventions for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![Crates.io](https://img.shields.io/crates/v/gize-db.svg)](https://crates.io/crates/gize-db)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-db` owns Gize's migration runner on top of
[SQLx](https://crates.io/crates/sqlx). See
[ADR-003](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-003-data-layer.md),
[ADR-011](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-011-migrations.md) and
[ADR-015](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-015-second-database.md).

- **`migrate`**: a synchronous wrapper around SQLx's runtime `Migrator`. It loads
  `migrations/*.sql`, tracks applied versions in `_sqlx_migrations`, and applies pending ones
  in order. It runs against both **PostgreSQL** and **SQLite** through SQLx's `Any` driver, so
  one code path serves either database. There is **no** risky runtime auto-migration: SQL is
  generated and reviewed, then applied on demand.

The database-specific SQL (column types, primary keys, placeholders) is generated from
`gize_core::Dialect`; the SQLx pool wiring lives in the generated app code, not here.

## Usage

```toml
[dependencies]
gize-db = "0.7"
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model, manifest, dialect, conventions |
| `gize-generator` | Codegen engine: safe writer, sync, plugins |
| `gize-templates` | Templates for the generated code |
| **`gize-db`** | Migrations, PostgreSQL and SQLite (this crate) |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

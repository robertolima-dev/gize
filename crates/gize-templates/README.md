# gize-templates

**Templates for the code the [Gize](https://github.com/robertolima-dev/gize) framework generates.**

[![Crates.io](https://img.shields.io/crates/v/gize-templates.svg)](https://crates.io/crates/gize-templates)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-templates` holds the templates that produce the Rust and SQL files Gize scaffolds:
projects, application modules, models, full CRUD slices, the built-in `users` resource, auth,
and the OpenAPI route module. They are consumed by
[`gize-generator`](https://crates.io/crates/gize-generator).

The generated output is **idiomatic Rust you own**: plain Axum handlers, plain SQLx queries,
plain SQL migrations. No hidden runtime, no reflection.

## Modules

- **`project`**: the `gize new` project skeleton (`Cargo.toml`, `main.rs`, router, state,
  config).
- **`module`**: an application module (`gize make app`).
- **`model`**: a model struct and its `CREATE TABLE` migration, dialect-aware (PostgreSQL or
  SQLite).
- **`crud`**: a complete layered CRUD resource (model, dto, repository, service, handler,
  routes, error, tests), with request validation and integrity-error mapping.
- **`user`**: the built-in `users` slice (password hashing, register/login, a self-service
  `GET /users/me`, and admin-gated management routes).
- **`auth`**: the generated `src/auth` module (Argon2 + JWT, route guard).
- **`openapi`**: the route module that serves `/openapi.json` and `/docs`.

Templates are Rust functions returning file contents. Per
[ADR-004](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-004-templates.md) the
internals may move to `minijinja` templates on disk without changing the generator API.

## Usage

```toml
[dependencies]
gize-templates = "0.7"
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model, manifest, dialect, conventions |
| `gize-generator` | Codegen engine: safe writer, sync, plugins |
| **`gize-templates`** | Templates for the generated code (this crate) |
| `gize-db` | Migrations (PostgreSQL and SQLite) |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

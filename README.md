<div align="center">

# Gize

**Productivity-first backend framework for Rust.**

Django-like velocity (scaffolding, conventions, generators, migrations)
without giving up Rust's guarantees, performance, or transparency.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-MVP-yellow.svg)](#project-status)

</div>

> The name comes from the plateau of the Great Pyramids: solid foundations meant to last.

Gize lets you go from an empty directory to a running, production-shaped CRUD API in
minutes. It generates **idiomatic Rust you own**: plain Axum handlers, plain SQLx
queries, plain SQL migrations. No hidden runtime, no reflection, no magic. Delete Gize and
you still have a working, idiomatic Rust codebase.

---

## Table of contents

- [Why Gize?](#why-gize)
- [Philosophy](#philosophy)
- [Project status](#project-status)
- [Installation](#installation)
- [Quickstart](#quickstart)
- [Command reference](#command-reference)
- [The generated project](#the-generated-project)
- [Built-in `users` resource](#built-in-users-resource)
- [Models and field types](#models-and-field-types)
- [Anatomy of a generated CRUD resource](#anatomy-of-a-generated-crud-resource)
- [The `gize.toml` manifest](#the-gizetoml-manifest)
- [Database and migrations](#database-and-migrations)
- [Runtime configuration](#runtime-configuration)
- [The safety model](#the-safety-model)
- [Architecture](#architecture)
- [Developing Gize](#developing-gize)
- [Roadmap](#roadmap)
- [Comparison](#comparison)
- [Contributing](#contributing)
- [License](#license)

---

## Why Gize?

Rust is excellent for backends (fast, safe, predictable) but **starting** a real project
is slow. There is no blessed layout, no scaffolding, no generators. Every team hand-wires a
router, a database pool, config, migrations, error handling, and repeats the
model → migration → repository → service → DTO → handler → routes → tests dance for every
resource.

Gize removes that day-one tax. One command scaffolds the project; one command generates a
full CRUD resource wired end-to-end; one command applies migrations; one command runs it.
What you get is normal Rust you can read and edit: Gize writes the boring 80%, you own
100%.

## Philosophy

- **Zero-cost abstractions.** Generated code is idiomatic and explicit.
- **No magic.** No hidden framework runtime; everything is a file you can read and diff.
- **You own the code.** Gize is an accelerator, not a cage: remove it and the app still
  works.
- **No unnecessary dependencies.** Every dependency is justified in an ADR.
- **Convention over configuration**, but customization is always possible.
- **Analyze, then decide, then implement.** Every significant choice is recorded as an ADR
  in [`ADR/`](./ADR).

## Project status

**Release Candidate (v0.8.x).** The Beta feature set (Admin UI, OpenAPI, multi-database, plugin
API) is complete and the **1.0 feature set is frozen** — the project is now hardening rather than
adding features. See [`STABILITY.md`](./STABILITY.md) for what is stable and the deprecation
policy, [`SECURITY.md`](./SECURITY.md) for the generated apps' security model, and the
[roadmap](./docs/roadmap.md) for what remains before 1.0 (benchmarks, docs). New frameworks such
as Actix are planned for v2.0.

| Command | State | What it does |
| --- | --- | --- |
| `gize new` | ✅ | Scaffold a project (built-in `users` + auth; `--database sqlite\|mysql`, `--openapi`, `--api-version`, `--ws`) |
| `gize make app` | ✅ | Scaffold a module and wire it in |
| `gize make model` | ✅ | Generate a model + migration |
| `gize make crud` | ✅ | Generate a full CRUD resource (incl. `belongs_to`) |
| `gize make migration` | ✅ | Blank (named) or model-diff `ALTER` migrations |
| `gize make admin` | ✅ | Generate a separate React admin SPA for all resources |
| `gize sync` | ✅ | Reconcile the project from `gize.toml` |
| `gize migrate` | ✅ | Apply / inspect migrations (Postgres, SQLite or MySQL) |
| `gize createadmin` | ✅ | Create the first admin user (interactive or CI) |
| `gize serve` | ✅ | Build and run the app (and the admin dev server, when present) |
| `gize fmt` / `gize check` | ✅ | rustfmt / clippy wrappers |
| `gize doctor` | ✅ | Diagnose environment/project |
| `gize <plugin>` | ✅ | Run a `gize-<name>` plugin (plugin API v0) |

New in Beta: **`gize make admin`** (Vite + React + TS SPA, data-driven from the manifest),
**OpenAPI** generation (`--openapi` → `/openapi.json` + `/docs`), **SQLite** behind a database
seam (`gize new --database sqlite`), and a **plugin API** (`gize-<name>` subcommands). See
[`docs/roadmap.md`](./docs/roadmap.md) for the full plan (MVP → Alpha → Beta → RC → v1.0 →
v2.0).

## Installation

**From crates.io:**

```bash
cargo install gize   # installs the `gize` binary
```

**From source:**

```bash
git clone https://github.com/robertolima-dev/gize
cd gize
cargo build --release
# the CLI binary is target/release/gize
cp target/release/gize /usr/local/bin/   # or add target/release to PATH
```

### Prerequisites

- Rust **1.85+** (edition 2024).
- **PostgreSQL** for running generated apps (`gize migrate` / `gize serve`).

## Quickstart

Build a working product API in four commands.

```bash
# 1. Scaffold a project (Axum + SQLx + PostgreSQL)
gize new shop
cd shop

# 2. Generate a full CRUD resource
gize make crud Product name:String price:i32 active:bool

# 3. Point at a database and apply the generated migration
cp .env.example .env                     # then edit DATABASE_URL if needed
export DATABASE_URL=postgres://localhost:5432/shop
createdb shop
gize migrate

# 4. Run it
gize serve
```

> Every new project already ships a **`users` resource** (model, CRUD and a migration with
> an `is_admin` flag) wired in, so after `gize migrate` you also have working
> `GET/POST/PUT/DELETE /users` endpoints. Pass `gize new shop --no-user` to skip it. See
> [Built-in `users` resource](#built-in-users-resource).

Now exercise the API:

```bash
# Create
curl -X POST localhost:8080/products \
  -H 'content-type: application/json' \
  -d '{"name":"Widget","price":1299,"active":true}'
# → {"id":"…","name":"Widget","price":1299,"active":true,"created_at":"…","updated_at":"…"}

curl localhost:8080/products            # list
curl localhost:8080/products/<id>       # show
curl -X PUT localhost:8080/products/<id> \
  -H 'content-type: application/json' \
  -d '{"name":"Widget","price":1500,"active":false}'   # update
curl -X DELETE localhost:8080/products/<id>            # delete → 204
```

Deleting a missing id returns `404`: the generated typed error maps `RowNotFound` to
`NOT_FOUND` for you.

## Command reference

Global flags on every **generating** command (`new`, `make …`):

| Flag | Effect |
| --- | --- |
| `--dry-run` | Print the planned file operations; write nothing. |
| `--force` | Overwrite files that already exist (otherwise they are skipped). |

### `gize new <name>`

Scaffolds a new project into a directory named `<name>`: `Cargo.toml`, `gize.toml`,
`.env.example`, `.gitignore`, and the full `src/` layout (see below). The project compiles
and serves immediately.

By default it also generates a built-in **`users`** resource (model, full CRUD and a
migration) already registered in `src/app/mod.rs` and `gize.toml`
(see [Built-in `users` resource](#built-in-users-resource)).

| Flag | Effect |
| --- | --- |
| `--no-user` | Scaffold the bare skeleton, without the built-in `users` resource. |
| `--database <db>` | Target database: `postgres` (default), `sqlite` or `mysql` ([ADR-015](./ADR/adr-015-second-database.md)). |
| `--openapi` | Generate an OpenAPI spec (`/openapi.json`) and docs UI (`/docs`) ([ADR-010](./ADR/adr-010-openapi.md)). |
| `--api-version <v>` | Mount CRUD routes under a versioned prefix. `--api-version 1` (or `v1`) serves them at `/api/v1/...`; omit for root-mounted routes ([ADR-016](./ADR/adr-016-api-versioning.md)). |
| `--ws` | Scaffold a WebSocket module (`src/app/ws/`) with a typed echo endpoint at `/ws` ([ADR-018](./ADR/adr-018-websocket.md)). |

With `--api-version`, the version is recorded in `gize.toml` under `[api]` and the whole API
(including the OpenAPI paths) is mounted under `/<prefix>/<version>`:

```toml
[api]
version = "v1"
prefix = "/api"
```

Without it, routes stay at the root (`/users`) exactly as before — existing projects are
unaffected.

### `gize make app <name>`

Scaffolds an application module `<name>` (the nine files of the module layout) with a
placeholder health route, then **registers it idempotently**: adds `mod <name>;` and
`.merge(<name>::routes())` to `src/app/mod.rs` and appends it to `[modules]` in
`gize.toml`. Re-running is a no-op.

### `gize make model <Name> field:Type …`

Generates `src/app/<table>/model.rs` (an `sqlx::FromRow` struct with `id` + timestamps) and
a `CREATE TABLE` migration. The module directory is the pluralized snake_case of the model
(`User` → `users`).

### `gize make crud <Name> field:Type …`

The headline command. Generates a complete, wired vertical slice for the resource:

```
src/app/<table>/
  mod.rs          model.rs      dto.rs        error.rs
  repository.rs   service.rs    handler.rs    routes.rs      tests.rs
migrations/<ts>_create_<table>.sql
```

…and registers the module in `src/app/mod.rs` and `gize.toml`. The result exposes working
`GET / POST / PUT / DELETE` endpoints backed by the database.

```bash
gize make crud Product name:String price:i32 active:bool
gize make crud Order   total:i64 paid:bool
gize make crud Article title:String body:String published:bool --dry-run
```

### `gize migrate [--status]`

Applies pending migrations from `migrations/*.sql` against `DATABASE_URL`, using SQLx's
migrator (tracked in the `_sqlx_migrations` table, ordered and idempotent).

```bash
gize migrate            # apply all pending
gize migrate --status   # list applied [x] vs pending [ ]
```

### `gize createadmin`

Creates the first admin user (`is_admin = true`) in the database — Gize's `createsuperuser`
([ADR-017](./ADR/adr-017-createadmin.md)). Reads the dialect from `gize.toml` and connects with
`DATABASE_URL`, so it works against Postgres, SQLite or MySQL. The password is hashed with
Argon2id (the same format the generated login verifies), so the created admin can sign in
immediately.

```bash
# Interactive: prompts for Email, Name, then a hidden (confirmed) Password.
gize createadmin

# Non-interactive (CI): email/name as flags, password from an env var — never an argument.
GIZE_ADMIN_PASSWORD=... gize createadmin \
  --email admin@example.com --name Admin --password-env GIZE_ADMIN_PASSWORD
```

A duplicate email is rejected, and if the `users` table has not been migrated yet the command
tells you to run `gize migrate` first.

### `gize serve`

Builds and runs the generated application (`cargo run`), streaming its logs. Reads
`DATABASE_URL` and `PORT` (default `8080`) from the environment.

When the project has a generated admin ([ADR-006](./ADR/adr-006-admin.md)), `gize serve` also
starts the **admin dev server** alongside the API ([ADR-019](./ADR/adr-019-serve-admin.md)). It
detects a package manager (pnpm → npm → yarn), runs the install on first use, and prints both
URLs; Ctrl-C stops both. This needs Node.js for the admin path only.

```bash
gize serve                # API + admin dev server (when an admin exists)
gize serve --api-only     # just the API
gize serve --admin-only   # just the admin dev server
gize serve --with-admin   # explicitly both
```

### `gize doctor`

Sanity-checks the environment and project: `cargo`/`rustfmt` availability, whether you are
inside a Gize project, and whether `DATABASE_URL` is set.

## The generated project

`gize new` produces this layout ([ADR-005](./ADR/adr-005-module-layout.md) explains every
directory):

```
shop/
├── Cargo.toml            # axum, tokio, sqlx, serde, uuid, chrono, tracing
├── gize.toml             # the project manifest
├── .env.example          # runtime config template
├── migrations/           # plain SQL migrations (…_create_users.sql ships by default)
└── src/
    ├── app/
    │   ├── mod.rs        # aggregates modules + merges their routers (has gize: markers)
    │   ├── users/        # built-in resource (unless --no-user); same layout as below
    │   └── <resource>/   # one directory per resource (added by make app / make crud)
    │       ├── mod.rs        # declares the module's files, re-exports routes
    │       ├── model.rs      # domain struct (sqlx::FromRow)
    │       ├── dto.rs        # request/response payloads
    │       ├── repository.rs # SQL access (SQLx)
    │       ├── service.rs    # business logic
    │       ├── handler.rs    # Axum handlers (HTTP ↔ service)
    │       ├── routes.rs     # this module's Router
    │       ├── error.rs      # typed error → IntoResponse
    │       └── tests.rs      # tests
    ├── config/           # typed runtime configuration
    ├── database/         # (reserved) pool/migration hooks
    ├── middleware/       # (reserved) Tower layers
    ├── shared/           # (reserved) cross-cutting utilities
    ├── router.rs         # top-level router; applies AppState
    ├── state.rs          # AppState { db: PgPool }
    └── main.rs           # entrypoint: build state, serve
```

The `// gize:modules` and `// gize:module-routes` markers in `app/mod.rs` are how `make
app` / `make crud` wire new modules in without disturbing your code. Leave them in place;
edit anything else freely.

## Built-in `users` resource

Every project starts with authentication-ready data. Unless you pass `--no-user`, `gize
new` scaffolds a full `users` resource (the same layered slice `gize make crud` produces)
and wires it into `src/app/mod.rs` and `gize.toml`. Its migration:

```sql
-- migrations/…_create_users.sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- **`email`** is `UNIQUE`.
- **`is_admin`** ships from day one as the flag a future admin panel / `gize-auth` can gate
  access on; it defaults to `false`.
- **`password`** is marked `#[serde(skip_serializing)]` on the model, so its hash is read
  from the database but **never serialized into API responses**.

> The `users` resource reuses the generic CRUD templates, so `CreateUser` currently accepts
> `password` as plain text and lets `is_admin` be set on create. Password hashing, dropping
> `is_admin` from the create DTO, and register/login endpoints are tracked in
> [`BACKLOG.md`](./BACKLOG.md) and land with `gize-auth`.

## Models and field types

Fields are given inline as `name:Type` (the canonical UX per
[ADR-012](./ADR/adr-012-cli.md)). Every model also gets an `id: Uuid` primary key and
`created_at` / `updated_at` timestamps automatically.

| Gize type | Aliases | Rust type | PostgreSQL type |
| --- | --- | --- | --- |
| `String` | `str` | `String` | `TEXT` |
| `bool` | `boolean` | `bool` | `BOOLEAN` |
| `i32` | `int` | `i32` | `INTEGER` |
| `i64` | `bigint`, `long` | `i64` | `BIGINT` |
| `f64` | `float`, `double` | `f64` | `DOUBLE PRECISION` |
| `Uuid` | (none) | `uuid::Uuid` | `UUID` |
| `DateTime` | `timestamp` | `chrono::DateTime<Utc>` | `TIMESTAMPTZ` |

Types are case-insensitive. Unknown types are rejected early with a helpful message.

## Anatomy of a generated CRUD resource

`gize make crud Product name:String price:i32 active:bool` produces a clean layered slice.
A taste of each layer:

**`model.rs`**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Product {
    pub id: uuid::Uuid,
    pub name: String,
    pub price: i32,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

**`repository.rs`** (SQLx runtime queries, compile without a database connection)
```rust
pub async fn create(pool: &PgPool, input: &CreateProduct) -> Result<Product, sqlx::Error> {
    sqlx::query_as::<_, Product>(
        "INSERT INTO products (name, price, active) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(input.name.clone())
    .bind(input.price)
    .bind(input.active)
    .fetch_one(pool)
    .await
}
```

**`handler.rs`** (plain Axum)
```rust
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateProduct>,
) -> Result<(StatusCode, Json<Product>), Error> {
    let item = service::create(&state.db, &input).await?;
    Ok((StatusCode::CREATED, Json(item)))
}
```

**`error.rs`** (typed, maps to HTTP)
```rust
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            Error::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (status, message).into_response()
    }
}
```

No macros hide any of this: it is exactly what you would write by hand
([ADR-007](./ADR/adr-007-macros.md) explains why Gize prefers generated source over
proc-macros).

## The `gize.toml` manifest

The declarative source of truth for a project's shape ([ADR-009](./ADR/adr-009-configuration.md)).
It drives the (planned) `gize sync` and is updated automatically by `make app` / `make
crud`. It never holds secrets: those live in the environment.

```toml
[project]
name = "shop"

[stack]
framework = "axum"
database  = "postgres"
orm       = "sqlx"

[features]
authentication = false
admin          = false
openapi        = false

[modules]
list = ["orders", "products"]
```

## Database and migrations

- Migrations are **plain SQL** files in `migrations/`, named
  `<version>_create_<table>.sql` ([ADR-011](./ADR/adr-011-migrations.md)).
- `gize make model` / `gize make crud` derive a `CREATE TABLE` from the model's fields;
  review and edit the SQL before applying.
- `gize migrate` applies them via the SQLx migrator, which records applied versions in
  `_sqlx_migrations` (ordered, idempotent). There is **no** risky runtime auto-migration.

```sql
-- migrations/…_create_products.sql
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    price INTEGER NOT NULL,
    active BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

> `gen_random_uuid()` is built into PostgreSQL 13+. On older versions run
> `CREATE EXTENSION pgcrypto;` once.

## Runtime configuration

Generated apps are 12-factor: runtime config comes from **environment variables**, not from
`gize.toml`.

| Variable | Default | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | (required) | PostgreSQL connection string |
| `PORT` | `8080` | HTTP listen port |

`gize new` writes a `.env.example` you can copy to `.env` for local development.

## The safety model

Generators never destroy your work ([ADR-012](./ADR/adr-012-cli.md)):

- Existing files are **skipped** unless you pass `--force`.
- `--dry-run` shows the full plan (`create` / `skip` / `update`) and writes nothing.
- Registry edits to `app/mod.rs` and `gize.toml` are **idempotent**: re-running a
  generator does not duplicate anything.

```text
$ gize make crud Product name:String price:i32 --dry-run
Generated CRUD for `Product`:
dry-run: no files written
  create  src/app/products/mod.rs
  create  src/app/products/model.rs
  …
  update  src/app/mod.rs (would register module + routes)
  update  gize.toml (would add module to [modules])
```

## Architecture

Gize is a Cargo workspace ([ADR-001](./ADR/adr-001-workspace.md)). The dependency direction
flows toward a framework-agnostic core:

```
gize ──> gize-generator ──> gize-templates ──┐
   │           │                              ├──> gize-core
   └──> gize-db ┴──────────────────────────────┘
```

| Crate | Responsibility |
| --- | --- |
| `gize-core` | Domain model: manifest, field/model specs, naming. No framework deps. |
| `gize-generator` | Codegen engine: pure `Plan`s, the safe `Writer`, idempotent registry edits. |
| `gize-templates` | The templates for generated projects, modules, models, and CRUD. |
| `gize-db` | Data-layer conventions + the SQLx migration runner. |
| `gize-macros` | Procedural macros (intentionally tiny; see ADR-007). |
| `gize` | The `gize` binary (clap). Orchestrates the above. |
| `gize-admin`, `gize-auth`, `gize-openapi`, `gize-testing` | Planned feature crates (placeholders today). |

Design decisions live in [`ADR/`](./ADR). Guides live in [`docs/`](./docs):

- [Getting started](./docs/getting-started.md) — empty directory to a running authenticated API.
- [Architecture](./docs/architecture.md) — how Gize generates code, and the shape of what it emits.
- [Cookbook](./docs/cookbook.md) — task-sized recipes (relationships, auth, OpenAPI, admin, …).
- [FAQ](./docs/faq.md) — is it an ORM, does it lock me in, is the auth production-ready, …
- [Migration guide](./MIGRATION.md) · [Stability policy](./STABILITY.md) · [Security](./SECURITY.md)
- [Vision](./docs/vision.md) · [MVP](./docs/mvp.md) · [Roadmap](./docs/roadmap.md)

## Examples

- [`examples/gize-chat`](./examples/gize-chat) — a real-time group chat: `gize new --ws` plus a
  `Message` resource, the built-in users/auth and an admin, on SQLite. It shows the Gize idea end
  to end — everything is generated, and turning the WebSocket echo starter into a real broadcast
  chat is a small, readable hand edit you own. Run it with `gize migrate && gize serve`, then open
  <http://localhost:8080>.
- [`examples/gize-healthcheck`](./examples/gize-healthcheck) — a tiny external plugin
  (`gize-<name>`) that generates a `/health` route ([ADR-008](./ADR/adr-008-plugins.md),
  plugin API v0).

## Developing Gize

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

CI (`.github/workflows/ci.yml`) enforces fmt + clippy (`-D warnings`) + tests on every push
and PR.

## Roadmap

- **MVP** (now): project + module + model + CRUD generators, migrations, serve.
- **Alpha**: `gize sync`, auth scaffolding, relationships, validation, migration diffing.
- **Beta**: admin UI (`gize make admin`), OpenAPI generation, a plugin API.
- **RC → v1.0**: API/codegen stability, benchmarks, security review, complete docs.

Full detail with acceptance criteria in [`docs/roadmap.md`](./docs/roadmap.md).

## Comparison

- **vs. Django / Rails / Laravel**: the same generator/convention productivity, but
  compiled, type-safe, and with generated code you can read.
- **vs. plain Axum**: Gize *uses* Axum; it adds the layout, scaffolding, and ecosystem
  Axum leaves to you. You still ship plain Axum code.
- **vs. Loco**: a different bet: transparent generated code over a heavy framework runtime,
  and SQLx-first over an ORM.

See [`docs/vision.md`](./docs/vision.md) for the detailed comparison.

## Contributing

Contributions are welcome. Because Gize is decision-driven, please open an ADR (or discuss
one) for anything architectural before implementing; see
[ADR-000](./ADR/adr-000-process.md) for the process. Keep `cargo fmt`, `cargo clippy -D
warnings`, and `cargo test` green.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE))
- MIT license ([LICENSE-MIT](./LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.

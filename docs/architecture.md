# Architecture

Two things have an architecture worth understanding: **Gize itself** (how it turns a manifest
into code) and **the project it generates** (the shape of the code you run). This document
covers both. Design rationale for each decision lives in the [ADRs](../ADR/).

## Guiding principle

Gize is a **code generator, not a runtime**. It emits idiomatic Axum + SQLx Rust that you own —
no hidden framework, no reflection, no macros doing invisible work. Delete Gize and a working,
transparent codebase remains. Everything below serves that principle.

## Part 1 — How Gize generates code

### The crates

Gize is a Cargo workspace ([ADR-001](../ADR/adr-001-workspace.md)). The CLI is the only supported
surface; the rest are implementation detail ([STABILITY.md](../STABILITY.md)).

| Crate | Responsibility |
| --- | --- |
| `gize` | The CLI: parses commands, orchestrates generation, runs migrations/serve. |
| `gize-core` | The manifest (`gize.toml`), field/model types, naming, the **dialect** seam. |
| `gize-generator` | The engine: `Plan`, the safe `Writer`, `registry` wiring, `sync` reconcile, `scaffold` entry points. |
| `gize-templates` | The idiomatic code templates (project, module, crud, user, auth, openapi, ws). |
| `gize-db` | The SQLx migrator ([ADR-003](../ADR/adr-003-data-layer.md)). |
| `gize-openapi` | OpenAPI 3.0 spec generation from the manifest ([ADR-010](../ADR/adr-010-openapi.md)). |
| `gize-admin` | The generated admin SPA shell ([ADR-006](../ADR/adr-006-admin.md)). |
| `gize-auth`, `gize-macros`, `gize-testing` | Placeholders / support (auth lives in templates per [ADR-013](../ADR/adr-013-auth.md)). |

### The generation pipeline

Every command that writes code follows the same pure, testable path:

```
gize.toml / CLI args
        │
        ▼
   scaffold::*        build a Plan (pure: a list of file-create ops, no I/O, no timestamps
        │             except where injected) — this is what --dry-run renders for free
        ▼
     Writer           applies the Plan to disk: never overwrites without --force, updates
        │             "registry" files (app/mod.rs, router) idempotently, runs rustfmt
        ▼
   files on disk      compile-, clippy- and rustfmt-clean by contract
```

- **`Plan`** is a value: a set of "create file X with contents Y" operations. Generation is a
  pure function `inputs → Plan`, which is why `--dry-run` is free and generation is unit-testable
  without touching the filesystem.
- **`Writer`** is the only thing that does I/O. It is conservative: a file that already exists is
  **skipped** unless `--force`. It formats the `.rs` files it writes with `rustfmt`
  ([ADR-020](../ADR/adr-020-format-generated-code.md)) so output is always `cargo fmt`-clean.
- **`registry`** edits the few "wiring" files (`src/app/mod.rs`, routers) idempotently — adding a
  module twice is a no-op.

### The manifest is the source of truth

`gize.toml` records `[project]`, `[stack]`, `[features]`, optional `[api]`, and one `[[module]]`
per resource (its fields and relationships) — [ADR-009](../ADR/adr-009-configuration.md). Any
generated project can be **reconstructed from the manifest alone**.

### `gize sync` — drift-aware reconciliation

`sync` builds the desired `Plan` from `gize.toml` and diffs it against the disk, classifying each
file as **missing** (create it), **drift** (exists but differs — *report, never overwrite without
`--force`*), or **unchanged**. This is what makes regeneration and version upgrades safe: your
hand edits are never silently clobbered ([MIGRATION.md](../MIGRATION.md),
[ADR-012](../ADR/adr-012-cli.md)).

### The dialect seam

Postgres, SQLite, and MySQL differ in SQL types, placeholders (`$1` vs `?`), UUID handling, and
`RETURNING` support. All of that is isolated behind a `Dialect` in `gize-core`
([ADR-015](../ADR/adr-015-second-database.md)); templates ask the dialect for the right fragment
rather than branching everywhere. Adding a database is implementing one seam.

### Plugins (v0)

`gize <name> …` shells out to a `gize-<name>` executable on `PATH`
([ADR-008](../ADR/adr-008-plugins.md)) — an explicitly unstable extension point for external
generators.

## Part 2 — The generated project

### Module layout ([ADR-005](../ADR/adr-005-module-layout.md))

```
src/
  main.rs          # builds the app, reads config, starts the server
  router.rs        # top-level router (nests app routes, optionally under /api/vN)
  state.rs         # AppState { db pool, ... } shared with handlers
  config/          # env-driven configuration (DATABASE_URL, PORT, ...)
  auth/            # Argon2id password hashing + JWT; require_auth / require_admin guards
  app/
    mod.rs         # registers every module + its routes (the registry-managed file)
    <resource>/
      mod.rs
      model.rs        # the struct, sqlx::FromRow
      dto.rs          # Create/Update payloads with `validator` rules
      repository.rs   # SQLx queries (runtime, compile without a DB connection)
      service.rs      # business logic between handler and repository
      handler.rs      # plain Axum handlers
      routes.rs       # this resource's routes + which guard protects them
      error.rs        # error → HTTP status mapping (404/409/422/500)
      tests.rs
migrations/          # plain, timestamped SQL
gize.toml
```

### Request flow

```
HTTP → router → routes.rs → [auth guard?] → handler.rs → service.rs → repository.rs → SQLx → DB
                                                 │
                                          dto.rs (validate)         error.rs (map failures → status)
```

A clean layered slice: handlers stay thin, business logic sits in the service, all SQL is in the
repository. You can read, edit, or delete any layer.

### Data & migrations ([ADR-011](../ADR/adr-011-migrations.md))

Migrations are plain SQL applied by the SQLx migrator, timestamped for stable ordering.
`gize make migration` (no name) **diffs** each model against its table and emits `ALTER TABLE ADD
COLUMN` for new fields; column drops are gated behind `--force`. `gize migrate --status` shows
applied vs pending. (Down-migrations/rollback are a post-1.0 item.)

### Authentication & authorization ([ADR-013](../ADR/adr-013-auth.md), [ADR-021](../ADR/adr-021-authorization.md))

Generated auth is **Argon2id** password hashing plus a stateless **JWT (HS256)** signed with
`GIZE_JWT_SECRET` from the environment. Two guards are emitted: `require_auth` (valid token) and
`require_admin` (token with the `is_admin` claim → 403 otherwise). Defaults:

- The **`users`** resource is admin-gated end to end (reads included); `register`/`login` are
  public and `register` forces `is_admin = false`.
- **Generic** resources: writes require `require_auth`, reads are public.
- Passwords are hashed on every write and never serialized in responses.

Authorization beyond "valid token / is admin" (ownership, roles, per-field policies) is yours to
add — see [SECURITY.md](../SECURITY.md).

### Optional features

Toggled in `gize.toml` `[features]` and reconciled by `sync`: **OpenAPI**
(`/openapi.json` + `/docs`), **Admin** (a generated React/Vite SPA), **WebSocket** (`src/app/ws/`),
and **API versioning** (routes under `/api/vN`, [ADR-016](../ADR/adr-016-api-versioning.md)).

## Further reading

- [Getting started](./getting-started.md) · [Cookbook](./cookbook.md) · [FAQ](./faq.md)
- [Vision](./vision.md) · [Roadmap](./roadmap.md) · [MVP](./mvp.md)
- [STABILITY.md](../STABILITY.md) · [SECURITY.md](../SECURITY.md) · [MIGRATION.md](../MIGRATION.md)
- The [ADRs](../ADR/) for the rationale behind every decision above.

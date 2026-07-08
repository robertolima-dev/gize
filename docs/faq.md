# FAQ

Short answers with pointers. See also [Getting started](./getting-started.md),
[Architecture](./architecture.md), and the [Cookbook](./cookbook.md).

## What is Gize?

A productivity-first backend framework for Rust: a CLI that scaffolds and grows an idiomatic
Axum + SQLx project — Django/Rails-style velocity without giving up Rust's guarantees or
transparency.

## Is Gize a framework or a code generator?

A **code generator**, not a runtime. It emits plain Rust you own — no hidden framework, no
reflection, no magic macros. There is no "Gize" in your dependency tree at runtime.

## Does it lock me in? What if I delete Gize?

You are not locked in. Delete the CLI and you keep a working, idiomatic Axum + SQLx codebase:
plain handlers, plain SQL queries, plain migrations. Gize is a tool you run, not a library you
depend on.

## Can I edit the generated code?

Yes — that is the point; you own it. `gize sync` is **drift-aware**: any file you change is
reported as drift and **never overwritten without `--force`**. Regeneration and version upgrades
preserve hand edits ([MIGRATION.md](../MIGRATION.md)).

## Is it an ORM?

No. The repository layer uses **SQLx** with plain SQL strings (runtime-checked, so they compile
without a live database). You see and control the SQL; there is no query-builder abstraction or
lazy-loading behind your back.

## How is this different from Loco, Django, or Rails?

Like those, Gize gives you scaffolding, conventions, generators and migrations. Unlike a runtime
framework, it generates **transparent code you own** rather than wrapping your app in framework
machinery. Compared to Django/Rails it targets Rust's performance and type safety; compared to
Loco it leans on codegen + a manifest you can regenerate from, rather than a runtime.
See [docs/vision.md](./vision.md).

## Which databases are supported?

**PostgreSQL** (default), **SQLite**, and **MySQL**, chosen with `gize new --database …`. SQL
types, placeholders, UUID handling and `RETURNING` sit behind a dialect seam
([ADR-015](../ADR/adr-015-second-database.md)).

## What database types can a model field have?

`String`, `bool`, `i32`, `i64`, `f64`, `Uuid`, `DateTime` (with aliases like `int`, `bigint`,
`float`, `timestamp`). Every model also gets `id: Uuid`, `created_at`, `updated_at` automatically.
Relationships use `field:belongs_to:target`.

## How do migrations work? Is there rollback?

Plain, timestamped SQL applied by the SQLx migrator. `gize make migration` with no name **diffs**
your models and emits `ALTER TABLE ADD COLUMN` for new fields (drops gated by `--force`);
`gize migrate --status` shows applied vs pending. Down-migrations/rollback are a **post-1.0**
item ([ADR-011](../ADR/adr-011-migrations.md)).

## Is the generated authentication production-ready?

The baseline is solid: **Argon2id** password hashing and stateless **JWT (HS256)** with the
secret from `GIZE_JWT_SECRET`, security-reviewed ([SECURITY.md](../SECURITY.md)). But
**authorization** beyond "valid token / is admin" is yours to add (ownership, roles, per-field
policies), and you must set a strong secret, configure CORS, and choose token storage. Read
[SECURITY.md](../SECURITY.md) before shipping.

## Why can't a non-admin read `/users`?

The `users` resource is **admin-gated by default** ([ADR-021](../ADR/adr-021-authorization.md)):
every route except `register`/`login` requires an admin token, so accounts and emails are not
exposed and no authenticated user can edit others. Generic resources keep the pragmatic default
(writes require auth, reads public). Create an admin with `gize createadmin`.

## Do I have to use the `users` resource?

No. `gize new --no-user` skips it entirely — bring your own users/auth. Generic resources still
get the `require_auth` guard for writes.

## How do I upgrade Gize?

`cargo install gize --force`, then `gize sync --dry-run` to preview and reconcile any drift. The
full flow, including how hand edits survive, is in **[MIGRATION.md](../MIGRATION.md)**.

## Does the OpenAPI spec drift from my routes?

No — it is generated from the same manifest the routes are ([ADR-010](../ADR/adr-010-openapi.md)),
so it matches by construction. Enable it with `gize new --openapi` or `openapi = true` in
`gize.toml`.

## Can I extend Gize with my own generators?

Yes, via the **v0 plugin API**: `gize <name> …` runs a `gize-<name>` executable on your `PATH`
([ADR-008](../ADR/adr-008-plugins.md)). It is explicitly unstable until a dedicated plugin-API
stabilization.

## What's stable? Can I rely on the generated code not changing?

The **CLI**, the **`gize.toml` schema**, and the **generated-code contract** (it compiles, is
clippy/rustfmt-clean, follows the documented layout and auth behavior) are the stable surface. The
**exact bytes** of generated code are *not* stable pre-1.0 — which is exactly why `sync` is
drift-aware. Full policy in [STABILITY.md](../STABILITY.md).

## What Rust version do I need?

**Rust 1.85+**, edition 2024, for both the CLI and generated projects.

## Where do I report a bug or ask for help?

Open an issue with your Gize version and a minimal reproduction. If an upgrade breaks a project in
a way not described in [STABILITY.md](../STABILITY.md) or the [CHANGELOG](../CHANGELOG.md), that is
a bug.

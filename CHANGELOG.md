# Changelog

All notable changes to Gize are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(pre-1.0: minor versions may introduce changes to generated output).

## [Unreleased] — Beta

Work toward the Beta (see `docs/roadmap.md`): Admin, OpenAPI, a second database, and a plugin
API. ADR-before-code: ADR-006 (admin), ADR-008 (plugins), ADR-010 (OpenAPI) and ADR-015
(second database) added.

### Added

- **Admin UI** (`gize make admin`, ADR-006). Generates a **separate** Vite + React +
  TypeScript SPA under `admin/`, data-driven from the manifest: one generic `Resource`
  component renders List/Create/Edit/Delete for every resource, with search, pagination, and
  forms validated by Zod schemas that mirror the backend `validator` rules. Auth uses the
  existing JWT login; the app reaches the API through a Vite dev proxy (`/api`), so the
  backend needs no CORS or other changes — the admin is a fully separable artifact.
  `admin/src/resources.ts` (the descriptors) is derived and refreshed by `gize make admin`
  and `gize sync`. Verified end-to-end in a headless browser (Playwright + Chrome): login,
  list, create (incl. a `belongs_to` FK) and delete all work against the running backend;
  the generated app builds under strict `tsc` and `vite build`.
- **OpenAPI generation** (ADR-010). `gize new --openapi` (or `features.openapi` in `gize.toml`)
  generates an OpenAPI 3.0.3 spec **from the manifest + DTOs** — the same source of truth the
  CRUD generator uses — so the spec matches the routes by construction (no drift). The app
  serves it at `GET /openapi.json` with a reference UI at `/docs`. The spec covers every
  resource's CRUD plus `users` register/login, marks write routes as bearer-secured, hides
  `password` from responses, and includes relationship FK columns. It is a derived artifact:
  `gize make crud` and `gize sync` refresh it automatically from the current manifest.

## [0.6.1] - 2026-07-06 — Alpha

The Alpha phase (see `docs/roadmap.md`): the manifest-driven workflow, authentication,
relationships and validation, verified end-to-end against PostgreSQL. The reference blog app
(`users` ← `posts` ← `comments`, auth-protected) is built through the CLI + `gize.toml` and
rebuilt from scratch reproducibly with `gize sync`. ADR-before-code: ADR-009 and ADR-011
revised; ADR-013 (auth) and ADR-014 (relationships) added.

### Added

- **Request validation and richer error mapping** in generated resources. DTOs now derive
  `validator::Validate`: `String` fields must be non-empty, and the `users` DTOs validate
  email format and a minimum password length. Handlers validate the payload before touching
  the database, returning **422 Unprocessable Entity** with a readable message. The resource
  error type gained `Conflict` and `Validation` variants, and a Postgres unique violation
  (SQLSTATE 23505, e.g. a duplicate email) now maps to **409 Conflict** instead of a generic
  500. Verified end-to-end: invalid email / short password → 422, duplicate email → 409.
- **Authentication, generated into every project** (ADR-013). `gize new` now emits a
  `src/auth` module with Argon2id password hashing and stateless JWT (HS256): `hash_password`
  / `verify_password`, `issue_token` / `verify_token`, and a `require_auth` middleware.
  Mutating routes (`POST`/`PUT`/`DELETE`) are guarded; reads stay public. The built-in `users`
  resource gains public `POST /users/register` and `POST /users/login` (returning a token) and
  hashes the password on every write. The signing secret is read from `GIZE_JWT_SECRET` (added
  to `.env.example`, reported by `gize doctor`) — never from `gize.toml`. Verified end-to-end
  against Postgres: guarded routes return 401 without a token and 201 with one; login rejects
  bad credentials; passwords never appear in responses. A security review (recorded in
  ADR-013) fixed a privilege-escalation where `register` accepted `is_admin`.
- **`belongs_to` relationships** in models (ADR-014). Declare a foreign key with a field
  token: `gize make crud Post title:String author:belongs_to:users`. The generated migration
  gets an `author_id UUID NOT NULL` column plus a `FOREIGN KEY (author_id) REFERENCES
  users(id)` constraint, and the model/DTOs carry `author_id`. Relationships are recorded
  under `[[module.belongs_to]]` in `gize.toml`, so `gize sync` rebuilds them; `sync` creates
  migrations in dependency order (a target table before the table that references it) and
  errors on a relationship cycle. Only `belongs_to`/one-to-many is supported in the Alpha; the
  reverse side is a plain query, and many-to-many is deferred.
- **`gize make migration` model diffing** (ADR-011). With no name, it now diffs each module's
  declared fields (`gize.toml`) against the columns in that table's existing migrations and
  emits `ALTER TABLE` migrations to reconcile. New columns are added automatically (as
  **nullable**, with a `-- TODO` to backfill and tighten — adding a `NOT NULL` column to a
  populated table would fail); dropped columns are **withheld** and only emitted with
  `--force` (a rename is indistinguishable from drop+add, so it is always surfaced for
  review). Column parsing reads only Gize's own generated SQL and never touches a database.
  Passing a name still generates a blank migration to edit by hand.
- **`gize sync`** — reconcile a project from `gize.toml` (ADR-009). It regenerates any
  declared module whose code is missing, creates a `CREATE TABLE` migration for any table
  that lacks one (idempotent — never spawns duplicates), and wires each module into
  `src/app/mod.rs`. Files that exist but differ from the manifest are reported as **drift**
  and left untouched unless `--force` is given; `--dry-run` previews without writing. This
  makes the manifest-driven workflow real: add a `[[module]]` block by hand (or clone a repo
  with only `gize.toml`) and `gize sync` scaffolds and wires the module. Generation goes
  through one shared code path (`scaffold::module_code`) with `gize new`/`make crud`, so a
  synced tree is byte-identical and drift-free.

### Changed

- **`gize.toml` is now a rich, per-module source of truth** (ADR-009 revision). Each module
  records its `fields` (the same `name:Type` grammar the CLI accepts) under `[[module]]`, so
  a project can be reconciled and rebuilt from the manifest alone. `gize new` and `gize make
  crud` write the module's full shape; the built-in `users` module records its fields too.
  The legacy names-only form (`[modules] list = [...]`) is still read for backward
  compatibility and upgraded to `[[module]]` on the next write.



### Added

- The CLI now auto-loads a project-local `.env` at startup (via `dotenvy`), so `gize
  migrate`, `gize serve` and `gize doctor` pick up `DATABASE_URL` / `PORT` without a manual
  `export`. `serve` spawns `cargo run`, which inherits the loaded values. A real
  environment variable still takes precedence over a `.env` entry.
- `gize doctor` reports whether a `.env` file is present.

### Fixed

- Migration timestamps now use nanosecond resolution instead of seconds. Generating two
  resources within the same second previously produced two migrations with the same sqlx
  version, which failed `gize migrate` with a `_sqlx_migrations_pkey` duplicate-key error.
  New stamps stay strictly greater than earlier ones, so ordering is preserved.

## [0.5.0] - 2026-07-05

### Added

- Integration and snapshot test coverage for the generator
  (`crates/gize-generator/tests/generation.rs`), closing the last MVP Definition of Done
  item ("covered by integration + snapshot tests in CI"):
  - Integration tests drive the real `Writer` against a temp directory and assert the
    generated tree, idempotent re-runs that preserve hand edits, `--force` overwrite and
    `--dry-run` no-write behavior, and that `make crud` lands a resource with its declared
    fields.
  - Snapshot (golden-file) tests pin the generated project skeleton and the full CRUD slice
    so template changes always surface in review. Refresh with `UPDATE_SNAPSHOTS=1 cargo test`.
  - Workspace test count: 36 → 42; `cargo clippy --all-targets -- -D warnings` and
    `cargo fmt --check` stay clean.

### Notes

- This release closes **Phase 1 (MVP)**: the MVP Definition of Done is met end to end.
  `gize sync` (Alpha) and `gize make admin` (Beta) remain out of MVP scope.

## [0.4.1] - 2026-07-05

### Changed

- Point the crate `homepage` to the project website
  (https://gize-rust-framework.vercel.app/en) instead of the GitHub repository.
- Remove em-dashes from the README and the crate description, rephrasing for flow.
- CLI `--dry-run` output now reads "dry-run: no files written" (no em-dash).

## [0.4.0] - 2026-07-05

### Added

- `gize new` now scaffolds a built-in `users` resource by default: model, full CRUD
  (dto, repository, service, handler, routes, error, tests) and a migration, wired into
  `src/app/mod.rs` and `gize.toml` automatically.
  - Minimal, authentication-ready fields: `id`, `name`, `email`, `password`, `is_admin`,
    plus `created_at` / `updated_at`.
  - `is_admin` (`BOOLEAN NOT NULL DEFAULT false`) is included from day one as the flag a
    future admin panel / `gize-auth` can gate access on.
  - `email` is `UNIQUE`; `password` is `#[serde(skip_serializing)]`, so its hash never
    leaks into API responses.
- `gize new --no-user` opts out of the built-in `users` resource and scaffolds the bare
  project skeleton.
- `Plan::extend` in `gize-generator` to compose a base plan with an optional add-on slice.

### Notes

- The generated project compiles and passes `cargo clippy -D warnings` end to end.
- Follow-ups tracked in `BACKLOG.md`: password hashing on create/update, removing
  `is_admin` from the `CreateUser` DTO, and register/login endpoints — all pending the
  `gize-auth` work.

[0.5.1]: https://github.com/robertolima-dev/gize/releases/tag/v0.5.1
[0.5.0]: https://github.com/robertolima-dev/gize/releases/tag/v0.5.0
[0.4.1]: https://github.com/robertolima-dev/gize/releases/tag/v0.4.1
[0.4.0]: https://github.com/robertolima-dev/gize/releases/tag/v0.4.0

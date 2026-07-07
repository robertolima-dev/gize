# ADR-017: `gize createadmin`

- **Status:** Accepted
- **Date:** 2026-07-07
- **Deciders:** Gize core team

## Context

Every generated project ships a built-in `users` resource with an `is_admin` flag (ADR-013)
and Argon2id password hashing. To use the admin UI (ADR-006) or any admin-gated route, a
project needs a first administrator — but there is no way to create one without hand-writing
SQL and an Argon2 hash. Django solves this with `createsuperuser`; Gize needs the equivalent.

Unlike the generators (`new`, `make …`), which only write files, this command **connects to
the database at runtime** and inserts a row. It must work against all three supported
databases (Postgres, SQLite, MySQL — ADR-015) and produce a row the generated app can read
back and authenticate against.

## Decision

Add `gize createadmin`, a runtime command that inserts one admin user.

**Input (two modes):**

- **Interactive** (default): prompt for `Email`, `Name`, then `Password` twice (hidden, via
  `rpassword`) and confirm they match.
- **Non-interactive** (CI/automation): `--email` and `--name` as flags; the password comes
  from an environment variable named by `--password-env` (default `GIZE_ADMIN_PASSWORD`).

The password is **never** accepted as a command-line argument (it would leak into shell
history and the process table). It is read from a hidden prompt or an env var only.

> The generated `users` schema has `name`, not `username` (ADR-013), so the command prompts for
> `Name`. This keeps the inserted row consistent with the model the app already serves.

**Validation:** email must look like an address (contains `@` and a `.` after it); password
must be at least 8 characters (mirroring the Zod/`validator` rule the generated project uses).

**Hashing:** the password is hashed with **Argon2id** using the same parameters as the
generated `src/auth` module, so the login flow verifies it. The hashing lives in the CLI crate
itself (a small `password` module) rather than in `gize-auth`: the published `gize` binary must
not depend on an unpublished crate, and the CLI is the only consumer. The generated project
keeps its own copy of the hashing code (the "you own the code" philosophy — the two are
intentionally independent).

**Persistence:** the command reads the dialect from `gize.toml` and connects with SQLx's
`AnyPool` (the same connection path as `gize migrate`), so one code path serves every database.
Because the `Any` driver passes SQL through unchanged, the `INSERT` uses the dialect's own
placeholder style (`$n` / `?n` / `?`) and id strategy:

- **Postgres:** omit `id`; the column default (`gen_random_uuid()`) supplies it.
- **SQLite / MySQL:** the app generates the id, bound as the UUID's **16 raw bytes**
  (`Vec<u8>`), which `Any` encodes as a BLOB / `BINARY(16)` — byte-for-byte what the generated
  app writes and reads for its `uuid::Uuid` id. `created_at`/`updated_at` use their column
  defaults.

`is_admin` is set to `true`. A duplicate email is rejected up front with a clear message
(the column is `UNIQUE` anyway). If the `users` table does not exist yet, the command fails
with guidance to run `gize migrate` first.

## Trade-offs

- (+) One command, one code path, all three databases (via `AnyPool` + the dialect seam).
- (+) The inserted row is byte-compatible with what the app reads, so login "just works".
- (+) Password never touches argv; supports both humans (hidden prompt) and CI (env var).
- (−) `gize-auth` now carries hashing that is *also* emitted into generated projects — a small,
  deliberate duplication in exchange for keeping generated code self-contained.
- (−) The command needs a live database, so its happy path is validated manually / against
  SQLite in tests, not in a pure unit test.

## Consequences

- The CLI gains a `password` module (`hash_password`, Argon2id) and depends on `argon2` and
  `rpassword`.
- `gize-db` gains `admin::create`, which connects via `AnyPool`, rejects duplicates, and
  inserts the dialect-appropriate row.
- The workspace `sqlx` gains the `mysql` feature so `AnyPool` can reach `mysql://` URLs (this
  also lets `gize migrate` run against MySQL).
- Acceptance: `gize createadmin` creates a working admin on SQLite (verified end-to-end: create
  then authenticate); Postgres/MySQL share the same code path.

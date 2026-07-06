# ADR-015: Second database behind the data seam

- **Status:** Accepted
- **Date:** 2026-07-06
- **Deciders:** Gize core team

## Context

The MVP/Alpha target PostgreSQL only. The Beta introduces a **second database** to prove the
data layer is not welded to Postgres and to widen adoption. `gize.toml` already records
`[stack] database`, but every generated migration and query currently assumes Postgres
dialect (`gen_random_uuid()`, `TIMESTAMPTZ`, `UUID`, `$1` placeholders, `SQLSTATE` codes).
We need a **seam** so the dialect-specific bits are chosen from the manifest, and a first
second target to validate it.

## Alternatives

**Which second database first:**

1. **SQLite.** Serverless (a file), zero setup, ideal for local dev and CI. Small dialect gap
   to cover. SQLx supports it. Best "prove the seam" target with the least operational cost.
2. **MySQL/MariaDB.** More common in production, but requires a running server to test, and a
   larger dialect gap (no native UUID type, different DDL/`ON CONFLICT`, `?` placeholders).
3. **Do both at once.** Doubles the dialect surface before the seam is proven; rejected.

**Where the seam lives:** in `gize-core`/`gize-db` (a `Dialect` abstraction) vs. scattered
`if postgres { … }` in templates (rejected — unmaintainable).

## Decision

Introduce a **`Dialect` seam** in `gize-db`/`gize-core` and implement **SQLite first**
(option 1); Postgres stays the default and is unchanged. MySQL is a later increment behind
the same seam.

- A `Dialect` captures the choices that differ per database: primary-key generation
  (`gen_random_uuid()` vs. app-side UUID / `blob`), column types (`UUID`/`TIMESTAMPTZ` vs.
  SQLite affinities), placeholder style, and integrity-error code mapping (Postgres `23505`/
  `23503` vs. SQLite's `UNIQUE`/`FOREIGN KEY` constraint errors) so the generated `error.rs`
  still returns `409`.
- `stack.database` in the manifest selects the dialect; the templates ask the dialect for the
  dialect-specific fragments instead of hard-coding Postgres.
- The generated `Cargo.toml` enables the matching SQLx feature; `state.rs` builds the right
  pool.
- `gize doctor`/`gize migrate` work against the selected database.

## Trade-offs

- (+) SQLite proves the seam with near-zero operational cost; great for tests/CI and quick
  starts.
- (+) A single `Dialect` abstraction keeps dialect logic in one reviewable place, not sprayed
  through templates.
- (+) Postgres path stays the default and untouched (no regression risk to shipped behavior).
- (−) SQLite's weaker typing (no native UUID/timestamptz, dynamic typing) means some mappings
  are approximations (UUID as `TEXT`/`BLOB`, timestamps as `TEXT`/`INTEGER`); documented.
- (−) SQLite concurrency/feature limits make it a dev/test target, not a heavy-prod one — the
  point here is the seam, not parity.
- (−) Every generator that emits SQL now routes through the dialect — a one-time refactor cost.

## Consequences

- `gize-db` gains a `Dialect` trait with a `Postgres` (default) and `Sqlite` implementation;
  `gize-templates` consults it for types, PK defaults, placeholders, and error-code mapping.
- Migration and query generation become dialect-aware; snapshots gain a SQLite variant.
- MySQL becomes an additive follow-up (new `Dialect` impl) with no further template churn.
- Acceptance (Beta): a project targeting SQLite generates, migrates and serves CRUD; Postgres
  remains the untouched default.

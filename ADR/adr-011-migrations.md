# ADR-011: Migration strategy

- **Status:** Accepted (revised 2026-07-06 for the Alpha phase — see "Revision")
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Gize generates models and must keep the database schema in sync. Migrations must be
inspectable (philosophy: no magic), versioned, ordered, and safe to apply/rollback.

## Alternatives

1. **SQLx migrations (SQL-first).** Plain `.sql` files in `migrations/`, applied by the
   SQLx migrator, tracked in a `_sqlx_migrations` table. Fully inspectable SQL.
2. **SeaORM migrations (Rust-DSL).** Migrations written as Rust code via SeaORM's migration
   framework. Tied to adopting SeaORM (rejected in ADR-003).
3. **Custom migration engine.** Full control, high cost, reinvents a solved problem.
4. **Auto-migrate from models at runtime.** Convenient but hides schema changes and is
   dangerous in production — rejected outright.

## Decision

Use **SQLx SQL-first migrations**, consistent with ADR-003:

- Migrations are timestamped `.sql` files under `migrations/` (e.g.
  `20260704120000_create_products.sql`), with optional paired `.down.sql` for rollback.
- `gize make migration` generates migration SQL — for `gize make model`/`crud`, it derives
  a `CREATE TABLE` from the model's fields; standalone, it creates an empty migration to
  edit.
- `gize migrate` applies pending migrations via the SQLx migrator; `gize migrate --status`
  shows applied vs. pending; rollback applies the `.down.sql`.
- Migrations are checked into version control and are the source of truth for the schema.

Model→migration is a **generation aid, not a runtime auto-sync**: the developer reviews and
edits the SQL before applying.

## Trade-offs

- (+) Plain SQL is transparent, portable, and DBA-reviewable.
- (+) Reuses SQLx's battle-tested migrator; no engine to maintain.
- (+) No risky runtime auto-migration.
- (−) Developers must understand SQL DDL (acceptable and arguably desirable).
- (−) Diff-based migrations (schema drift → migration) are non-trivial; MVP generates
  create-table only, with smarter diffing deferred to Alpha.

## Consequences

- `gize-db` owns the migrator integration and the field-type → SQL-type mapping.
- CI runs migrations against a Postgres service before integration tests.
- Rollback support (`.down.sql`) is generated where derivable; otherwise stubbed with a TODO.

## Revision (Alpha) — model-change diffing

The MVP generated `CREATE TABLE` only. The Alpha adds **diff-based migrations**: when a
module's fields in `gize.toml` (the source of truth, ADR-009 revision) have drifted from the
schema described by the existing migrations for its table, `gize make migration` can emit the
`ALTER TABLE` needed to reconcile them.

**Diff source.** We diff the manifest's desired columns against the columns implied by the
table's already-generated migrations (the checked-in SQL is the schema of record) — not
against a live database connection. This keeps diffing offline, deterministic, and testable,
and avoids diverging behaviour between a developer with and without a reachable DB.

**What is generated automatically vs. gated:**

- **Additive changes → automatic.** A new field in the manifest with no matching column emits
  `ALTER TABLE <t> ADD COLUMN <c> <type>`. New nullable columns, or columns with a default,
  are safe; a new `NOT NULL` column without a default is emitted as nullable with a `-- TODO`
  note (backfill then tighten), never a data-losing default guess.
- **Destructive changes → gated, never automatic.** A column present in the schema but absent
  from the manifest is a potential `DROP COLUMN`, and a type change is a potential rewrite.
  These are **reported**, not written, unless the developer confirms with `--force` (mirroring
  the safety model of `gize sync`, ADR-009). Rename is indistinguishable from drop+add at the
  column level, so it is always surfaced for human decision rather than inferred.

**Timestamp format.** Migration filenames/`_sqlx_migrations` versions move from the interim
nanosecond stamp (`{nanos:020}`, introduced in 0.5.1 to break same-second collisions) to a
calendar stamp `YYYYMMDDHHMMSS` plus a short monotonic disambiguator when two are generated in
the same second. Calendar stamps are human-readable, sort correctly, and stay strictly greater
than every earlier nanosecond stamp (which are ~1.7e18, far below year-3000 calendar stamps),
so ordering across the upgrade is preserved.

**Still not runtime auto-migration.** Diffing produces a reviewable `.sql` file; nothing is
applied until `gize migrate`. The developer remains the owner of the DDL.

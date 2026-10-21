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

**Timestamp format.** We keep the nanosecond stamp (`{nanos:020}`, from 0.5.1). A
human-readable calendar stamp (`YYYYMMDDHHMMSS`) was considered and **rejected**: a calendar
stamp for any realistic date (~2e13) is *smaller* than a current nanosecond-since-epoch stamp
(~1.7e18), so switching would make Alpha-generated migrations sort *before* the nanosecond
migrations already present in a 0.5.1 project. sqlx applies migrations in version order and
refuses out-of-order additions, so the switch would break `gize migrate` on existing projects.
Nanosecond stamps are unique, monotonic, and correctly ordered, so they stay.

**Still not runtime auto-migration.** Diffing produces a reviewable `.sql` file; nothing is
applied until `gize migrate`. The developer remains the owner of the DDL.

## Revision (2026-10-20) — replay forward schema history

Use `sqlparser` 0.53 with the selected database dialect to parse checked-in migrations.
This dependency replaces substring/line matching, which cannot safely handle comments,
quoted identifiers, expressions, or multiple statements. Version 0.53 resolves with the
workspace's Rust 1.85 compatibility constraint; it adds no database connection or runtime
dependency to generated applications. A hand-written SQL lexer was rejected because it would
duplicate grammar maintenance and retain the original correctness risks.

Replay forward migrations by numeric version, skipping `.down.sql` and rejecting duplicate
versions. Maintain table/column names, SQL types and nullability through CREATE, ADD, DROP,
column/table renames, PostgreSQL ALTER COLUMN and MySQL MODIFY/CHANGE COLUMN. Explicit
CREATE/INSERT/DROP/RENAME rebuilds cover SQLite changes. SQL is parsed, never executed here.

Unknown SQL or operations that cannot be modelled conservatively stop automatic diffing
before any files are written, with the migration filename and statement context. This is a
bounded schema interpreter, not a general SQL execution engine. Schema-qualified names,
dynamic SQL, CREATE TABLE AS/LIKE, table/column CASCADE, and MySQL executable comments
are currently unsupported. Unknown statements also stop replay when they concern another
table, because their effects cannot safely be assumed local. Common auxiliary statements
(CREATE EXTENSION/VIEW/FUNCTION/TRIGGER/TYPE, COMMENT, GRANT/REVOKE) are accepted without
executing their bodies. ADD/DROP/RENAME CONSTRAINT are accepted: adding a primary key
marks its columns required; dropping it preserves NOT NULL, as in PostgreSQL/MySQL.
FK, CHECK and UNIQUE constraints do not change the tracked column metadata. Index changes, ordinary data
INSERT/UPDATE/DELETE, BEGIN/COMMIT, and SQLite foreign-key PRAGMAs with numeric values
(`foreign_keys = 0/1`, `defer_foreign_keys = 0/1`) do not change column shape. The selected
parser version rejects bare ON/OFF PRAGMA values; use the equivalent numeric spelling.
`DROP FUNCTION` and `DROP TRIGGER` are accepted without `CASCADE`; cascading drops
remain rejected because dependent objects may change the tracked schema.
`DROP TYPE` is accepted on the same terms (without `CASCADE`, the database refuses to drop a
type a column still uses). `DROP EXTENSION` and `ALTER TYPE` (including the usual
`ALTER TYPE ... ADD VALUE` for PostgreSQL enums) are not parsed by sqlparser 0.53 and
therefore still block offline diff; keep enum changes out of the replayed history or
generate later model changes with named migrations.
Named manual migrations and `gize migrate` remain available for unsupported history.

Type differences from generator defaults produce warnings for every module. Existing SQL
types are preserved, including under `--force`; independent additions and removals can still
be generated. VARCHAR lengths, timestamp timezones and numeric precision may be intentional,
so a named conversion migration is required only when the developer wants to change them.
Warnings do not certify Rust/SQL decoding compatibility. SQL aliases are normalized, but
identical SQL representations (such as SQLite i32/i64) cannot be distinguished from SQL alone.
Indexes, defaults and foreign-key constraints are not inferred from the manifest.

Replay is loaded once per command into a shared immutable `SchemaHistory`. Modules without
fields or relations and without a table are treated as code-only modules. A table that exists
with no declared fields still has its last explicit columns considered for gated drops.
`known_columns_for_dialect` supports dialect-specific callers; the legacy `known_columns`
helper remains explicitly PostgreSQL-only. The next release is prepared as 0.9.0 because
public Rust API signatures and struct fields changed; no publication is implied.

Added columns stay nullable for populated-table safety. A subsequent diff warns when a
required Rust field still maps to a nullable column; it does not repeat the ADD or claim full
alignment. The developer must backfill and tighten the constraint (or rebuild on SQLite).
Replay tracks the constraint, not the contents or completion of the backfill itself.

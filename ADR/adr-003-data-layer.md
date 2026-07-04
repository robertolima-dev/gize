# ADR-003: Data layer — SQLx vs SeaORM

- **Status:** Accepted
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Gize generates models, repositories, and migrations. The data-access choice determines how
"magic" vs. explicit the persistence layer is, how migrations work, and how close the
developer stays to SQL.

## Alternatives

1. **SQLx.** Async, compile-time-checked SQL (against a real DB), not an ORM. You write
   SQL; results map to structs. Migrations via `sqlx migrate`.
2. **SeaORM.** Full async ORM on top of SQLx: entities, relations, query builder, active
   record-ish API. More abstraction, more "magic", more generated entity glue.
3. **Diesel.** Mature, synchronous core (async story is younger), powerful typed DSL, but a
   heavier macro/DSL learning curve and a different async model.
4. **Raw SQL, no library.** Maximum control, minimum help; reinvents pooling/mapping.

## Decision

**SQLx-first**, PostgreSQL for the MVP. Gize generates:

- Model structs that derive `sqlx::FromRow`.
- Repositories containing explicit, reviewable SQL queries.
- SQL migration files applied via SQLx's migrator (see ADR-011).

SeaORM is reconsidered (not adopted) if/when relationship-heavy generation demands it; the
`gize-db` seam keeps that option open.

## Trade-offs

- (+) Explicit SQL aligns with Gize's "no magic / developer owns the code" philosophy.
- (+) Compile-time query checking catches errors early.
- (+) Thin abstraction → idiomatic, easy-to-read generated repositories.
- (+) Migrations are plain SQL — portable, inspectable, DBA-friendly.
- (−) More generated SQL boilerplate than an ORM's query builder.
- (−) Relationships/eager-loading are manual vs. SeaORM's relations.
- (−) Compile-time checking needs a reachable DB or offline query cache (`sqlx prepare`).

## Consequences

- `gize make model` / `gize make crud` emit SQLx repositories + SQL migrations.
- CI must provide a Postgres service (or use `sqlx` offline mode) for compile-time checks.
- `gize-db` wraps pool creation and exposes conventions, keeping generated code thin.
- Second databases (MySQL/SQLite) are a Beta+ concern behind the same seam.

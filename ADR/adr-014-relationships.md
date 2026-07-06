# ADR-014: Model relationships (belongs_to / one-to-many)

- **Status:** Accepted
- **Date:** 2026-07-06
- **Deciders:** Gize core team

## Context

Real apps have related tables: a post belongs to a user, a comment belongs to a post. The MVP
codegen only understands flat scalar fields, so the Alpha reference app (a blog) cannot be
built from the CLI/manifest without a way to express relationships. We need the smallest
relationship model that lets the blog exist, without importing an ORM's identity map or
lazy-loading — that would violate the "no magic, you own the code" principle (ADR-003/004).

## Alternatives

1. **`belongs_to` only (FK column), one-to-many by inference.** Express the owning side; the
   reverse (`user has many posts`) is just a query. A `belongs_to` on B targeting A means a
   `a_id` FK column on B. Minimal, maps directly to SQL.
2. **Full relationship DSL** (has_many, has_one, many-to-many with join tables, eager loading).
   Powerful but large; risks the "magic ORM" we rejected, and most of it is unneeded for Alpha.
3. **No first-class relationships; hand-write the FK field as a plain `uuid` column.** Works
   today but the generator can't emit the `FOREIGN KEY` constraint or the typed accessor, and
   the manifest can't reason about it for `sync`.

## Decision

Support **`belongs_to` only** in the Alpha (option 1). One-to-many is the natural reverse and
needs no stored declaration — it is a `WHERE a_id = $1` query.

- **CLI syntax:** a relationship is expressed as a field token
  `author:belongs_to:users` (field name `author`, relation kind `belongs_to`, target module
  `users`). This reuses the existing `name:Type` token stream; `belongs_to:<target>` is a new
  "type".
- **Manifest:** stored under the module as `[[module.belongs_to]]` with `target` (and the
  derived FK field name), per the ADR-009 revision.
- **Codegen:**
  - Model gets a `<name>_id: uuid::Uuid` field (targets are keyed by `id UUID`).
  - Migration emits the column plus `FOREIGN KEY (<name>_id) REFERENCES <target>(id)`. The
    referenced table must be created first — generation/`sync` orders migrations so the target
    exists (topological by dependency; a cycle is an error).
  - DTOs accept the FK id; no automatic join or nested serialization (no lazy-load magic).
- **many-to-many / has_one** are explicitly deferred (a later ADR if demand appears).

## Trade-offs

- (+) Directly expressible in SQL; the generated code is an ordinary FK column + constraint.
- (+) No identity map, no N+1 surprises, no lazy proxies — the developer writes the join query
  when they want one.
- (−) The reverse side (`has_many`) has no generated helper in Alpha; it's a manual query.
- (−) Ordering/cycle handling adds complexity to the generator and `sync` planner.
- (−) Only UUID-keyed targets are supported initially (consistent with the generated `id`).

## Consequences

- `gize-core` grows a `Relation { field, kind: BelongsTo, target }` type and validates that
  every target is a known module.
- `gize-templates` emits FK columns/constraints and includes the FK in model/DTO.
- `gize sync` and migration-diffing must order operations by dependency (targets before
  dependents).
- The blog reference app (users → posts → comments) becomes expressible end-to-end.

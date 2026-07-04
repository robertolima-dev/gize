# ADR-007: Procedural macros — when and why

- **Status:** Accepted
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Rust proc-macros are powerful but can hide behavior — directly at odds with Gize's "no
magic, developer owns the code" philosophy. Yet some ergonomics (validation, route
registration, admin field metadata) are painful without them. We need a principled rule.

## Decision

**Prefer generated source code over macros.** Use proc-macros only when *all* of the
following hold:

1. The alternative is significant, repetitive boilerplate in every generated file.
2. The macro's expansion is small, predictable, and documented (ideally showable via
   `cargo expand`).
3. It does not obscure control flow or hide I/O.

Concretely, allowed macro uses:
- **Derive macros for metadata** (e.g. `#[derive(GizeModel)]` producing table name, column
  list, `FromRow` glue) — declarative, inspectable.
- **Validation derives** on DTOs (or reuse `validator`) — declarative constraints.

Disallowed / avoided:
- Macros that generate whole handlers, routing tables, or business logic. That is the
  generator's job, producing *visible* files the developer can read and edit.
- Attribute macros that rewrite function bodies or inject runtime behavior.

## Alternatives

1. **Macro-heavy framework** (Rocket-style). Ergonomic but hides code — rejected as the
   primary model.
2. **No macros at all.** Maximally explicit but forces verbose repeated glue in every file.
3. **Hybrid (chosen):** codegen for structure, thin derives for declarative metadata.

## Trade-offs

- (+) Keeps generated code readable and greppable; preserves the "delete Gize and keep a
  working app" property (derives come from a small, optional `gize-macros` dep).
- (−) A generated app depending on `gize-macros` derives is not 100% dependency-free;
  mitigated by keeping derives optional and their output equivalent to hand-written code.

## Consequences

- `gize-macros` stays intentionally small; each macro has a documented expansion and tests
  asserting the expansion.
- Anything that would hide logic is implemented as generated source instead.

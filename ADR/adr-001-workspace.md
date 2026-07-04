# ADR-001: Cargo workspace layout

- **Status:** Accepted
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Gize is an ecosystem, not a single binary: a CLI, a codegen engine, templates, a data
layer, macros, and optional feature crates (admin, auth, openapi, testing). We need a
repository structure that supports independent compilation, clear boundaries, and
incremental builds.

## Alternatives

1. **Single crate.** Everything in one binary crate. Simple to start, but couples
   unrelated concerns, bloats compile times, and prevents publishing pieces independently.
2. **Multi-repo (one repo per crate).** Maximum isolation, but painful cross-crate changes,
   version skew, and heavy contributor overhead early on.
3. **Cargo workspace (mono-repo, many crates).** One repo, many crates sharing a lockfile
   and CI, published independently when ready.

## Decision

Use a **Cargo workspace** with crates under `crates/`:

```
gize-core        # domain model, conventions, shared types (no framework deps)
gize-generator   # codegen engine: template rendering + safe file writes
gize-templates   # the templates for generated projects/resources
gize-db          # data-layer abstraction (SQLx-based)
gize-macros       # proc-macros (used sparingly; see ADR-007)
gize-cli         # the `gize` binary (clap), orchestrates the above
gize-admin       # optional: admin generator (later phase)
gize-auth        # optional: auth scaffolding (later phase)
gize-openapi     # optional: OpenAPI generation (later phase)
gize-testing     # test utilities for generated apps
```

Plus top-level `examples/`, `docs/`, `website/`, `ADR/`, `README.md`.

**Dependency direction:** `gize-cli` → depends on `gize-generator`, `gize-core`,
`gize-db`; `gize-generator` → `gize-core`, `gize-templates`. `gize-core` depends on
nothing framework-specific. Feature crates depend on `gize-core` only. No cycles.

## Trade-offs

- (+) Independent compilation & publishing; clear seams; framework-agnostic `gize-core`.
- (+) One lockfile, one CI, atomic cross-crate refactors.
- (−) More upfront crate boilerplate than a single crate.
- (−) Requires discipline to keep the dependency graph acyclic.

## Consequences

- `gize-core` is the abstraction seam that keeps a future Actix/other target feasible
  (ADR-002).
- Optional crates (admin/auth/openapi) can lag the MVP without blocking it.
- The workspace `Cargo.toml` pins shared dependency versions via `[workspace.dependencies]`.

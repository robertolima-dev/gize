# ADR-005: Generated project & module layout

- **Status:** Accepted
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Conventions over configuration require a single, justified project layout. It must scale
from one module to many, keep concerns separated, and read naturally to Rust developers.

## Decision

Generated projects use this layout:

```
src/
  app/
    <module>/
      mod.rs         # re-exports; module wiring
      model.rs       # domain struct(s), sqlx::FromRow
      dto.rs         # request/response DTOs + validation
      repository.rs  # SQL access (SQLx)
      service.rs     # business logic, orchestrates repository
      handler.rs     # Axum handlers (HTTP <-> service)
      routes.rs      # Router for this module
      admin.rs       # optional admin registration (later)
      error.rs       # module-specific error type
      tests.rs       # unit/integration tests
  config/            # typed config loading (env + gize.toml)
  database/          # pool creation, migration runner hooks
  middleware/        # Tower layers (tracing, auth, etc.)
  auth/              # auth wiring (later phase)
  jobs/              # background jobs (later phase)
  events/            # domain events (later phase)
  shared/            # cross-cutting types/utilities
  router.rs          # top-level router; mounts each module's routes
  state.rs           # AppState (pool, config, shared services)
  lib.rs             # library crate root
  main.rs            # binary entry: build state, run server
```

### Rationale per layer

- **`app/<module>/` with layered files** (model→dto→repository→service→handler→routes)
  enforces a clear request flow and testable boundaries; mirrors what disciplined Axum
  teams already do by hand.
- **`repository` vs `service`** separates persistence from business logic, so services are
  unit-testable without HTTP or a DB double at the wrong layer.
- **`dto` separate from `model`** keeps the wire format decoupled from the persistence
  struct — critical for API evolution and validation.
- **`error.rs` per module** enables precise `IntoResponse` mapping instead of a giant
  global error enum.
- **`config`, `database`, `middleware`, `state`, `router`** are the app's backbone, wired
  once by `gize new`.
- **`auth`, `jobs`, `events`** exist as directories from day one (empty/placeholder) so the
  layout doesn't churn when those features land.

## Alternatives

- **Flat module (everything in one file).** Fast for demos, unmaintainable at scale.
- **Hexagonal/ports-and-adapters everywhere.** More layers than a CRUD app needs; risks
  overengineering (explicitly discouraged by the project philosophy).

## Trade-offs

- (+) Predictable, scalable, testable; matches idiomatic hand-written Axum apps.
- (−) More files per module than a minimalist would write for a tiny app; acceptable given
  the target is real, growing projects.

## Consequences

- `gize make app` generates this file set and wires `app/mod.rs` + `router.rs`.
- `gize make crud` fills `repository/service/dto/handler/routes/tests`.
- Templates in `gize-templates` are organized to mirror this tree.

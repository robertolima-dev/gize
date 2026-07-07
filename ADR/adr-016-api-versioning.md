# ADR-016: API route versioning

- **Status:** Accepted
- **Date:** 2026-07-07
- **Deciders:** Gize core team

## Context

Generated CRUD routes are currently mounted at the root: `/products`, `/products/:id`, etc.
Every module's `routes()` is merged into `app::routes()` (via the `// gize:module-routes`
marker), and `router::build()` merges that into the top-level router:

```rust
Router::new()
    // gize:routes (do not remove this marker)
    .merge(app::routes())
    .with_state(state)
```

A public API benefits from a stable version prefix (`/api/v1/...`) so the surface can evolve
without breaking clients. We need to decide **where** the version lives (per project vs per
resource) and **how** it is expressed so existing projects keep working unchanged.

## Decision

Versioning is **per project**, opt-in at creation time.

```bash
gize new shop --api-version 1     # or --api-version v1
```

- The version is **normalized** to a `v`-prefixed segment: `1` and `v1` both become `v1`,
  `2` becomes `v2`.
- It is recorded in `gize.toml` under a new optional `[api]` table:

  ```toml
  [api]
  version = "v1"
  prefix = "/api"
  ```

- `router::build()` then **nests** the app under `<prefix>/<version>` instead of merging at
  the root:

  ```rust
  Router::new()
      // gize:routes (do not remove this marker)
      .nest("/api/v1", app::routes())
      .with_state(state)
  ```

  So `products` becomes `/api/v1/products`.

- **Without `--api-version`, nothing changes.** No `[api]` table is written, and
  `router::build()` keeps the byte-identical `.merge(app::routes())` at the root. Existing
  projects are unaffected.

### Why per project, not per resource

Routes are wired uniformly: each module's `routes()` is merged together in `app::routes()`
through a single marker, and mounted once in `router::build()`. A single mount point makes a
project-wide prefix a one-line change. Versioning individual resources under different
prefixes would fracture that uniform wiring (mixing `merge`/`nest` per module) for little
real-world benefit — an API almost always versions its whole surface at once. A per-resource
`make crud --version` override is therefore **deferred** (not part of this ADR); if it is ever
added it will be an explicit, additive opt-in that does not change the project-level default.

## Trade-offs

- (+) One predictable prefix for the whole API; trivial, single-point implementation.
- (+) Fully backward compatible — unversioned projects generate identical output.
- (+) The prefix is data in `gize.toml`, so `gize sync` and future generators (WebSocket
  routes, OpenAPI paths) can read it and stay aligned.
- (−) No per-resource versioning (judged an anti-pattern for the common case; deferred).
- (−) Changing a project's version after creation is a manual `gize.toml` + `router.rs` edit
  for now (a `gize` command for it can come later).

## Consequences

- `gize-core` gains an optional `Api { version, prefix }` on the `Manifest`, with
  `Api::from_version` doing the `1`/`v1` → `v1` normalization and `Api::mount_path` returning
  `/api/v1`.
- `project::router_rs` takes the optional mount path and emits `nest` vs `merge` accordingly.
- The OpenAPI document (ADR-010), when enabled, prefixes its paths with the same mount path so
  the spec matches the served routes.
- WebSocket routes (ADR-018) mount under the same prefix when a project is versioned.

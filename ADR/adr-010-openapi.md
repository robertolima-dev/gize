# ADR-010: OpenAPI generation

- **Status:** Accepted
- **Date:** 2026-07-06
- **Deciders:** Gize core team

## Context

The Beta ships API documentation. An OpenAPI spec makes the generated API self-describing:
consumable by Swagger/Scalar UI, client generators, and the admin (ADR-006). The central risk
is **drift** — a spec that no longer matches the real routes is worse than no spec. So the
question is where the spec's truth comes from.

## Alternatives

1. **Generate from the manifest + DTOs (single source of truth).** Gize already owns
   `gize.toml` (modules, fields, relationships) and generates the DTOs and routes from it. The
   same source produces the spec, so paths, schemas and status codes match the routes **by
   construction**.
2. **`utoipa` derive macros on the generated types.** Idiomatic and popular: annotate the
   generated structs/handlers with `#[derive(ToSchema)]` / path macros. But the annotations
   live scattered across the generated code and can drift from the actual router wiring; it
   also couples generated code to `utoipa`.
3. **Hand-written / external spec.** Maximum control, guaranteed to drift, rejected.

## Decision

**Generate the OpenAPI spec from the manifest + DTOs** (option 1), in a `gize-openapi` crate.

- The generator walks `gize.toml`'s modules and their fields/relationships (the same data the
  CRUD and admin generators use) to emit paths (`GET/POST/PUT/DELETE /<table>`, `/<table>/{id}`,
  and the `users` `register`/`login`), request/response schemas from the DTOs and model, and
  the status codes Gize actually returns (`200/201/204/401/404/409/422`).
- The spec is emitted as a checked-in `openapi.json` (reviewable, diffable) and served by the
  app at `GET /openapi.json`, with a bundled Scalar/Swagger UI at `/docs`. Wiring is gated by
  `features.openapi` in the manifest and reconciled by `gize sync`.
- Security scheme: HTTP bearer (JWT), matching ADR-013; write operations are marked as
  requiring it.

## Trade-offs

- (+) **No drift by construction** — one source of truth (the manifest) feeds routes, DTOs,
  admin and spec together.
- (+) Deterministic, testable output; the spec is a plain artifact the developer owns.
- (+) No new annotations to sprinkle through generated code; no `utoipa` coupling.
- (−) Gize's OpenAPI generator must track the templates: when a route/DTO convention changes,
  the spec generator changes too. Mitigated by a **spec-vs-router test** (every path in the
  spec resolves in the generated router, and vice versa) in the generated project.
- (−) Custom, hand-written routes the developer adds outside the generator are not in the spec
  until they extend it — documented as expected.

## Consequences

- `gize-openapi` reads `gize_core`'s manifest/model and renders the spec; it depends on no web
  framework (pure data → JSON).
- The generated app serves `/openapi.json` + `/docs` when `features.openapi` is on.
- A generated integration test asserts spec/route parity, catching drift in CI.
- Acceptance (Beta): the spec validates against the OpenAPI schema and matches the generated
  routes.

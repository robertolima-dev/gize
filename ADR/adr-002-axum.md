# ADR-002: Axum as the initial web framework

- **Status:** Accepted
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Generated apps need an HTTP framework. The choice shapes the ergonomics of every generated
handler, the middleware story, and the ecosystem we inherit.

## Alternatives

1. **Axum.** Tokio/Tower-based, minimal, type-safe extractors, huge momentum, composes
   with the `tower` middleware ecosystem. Maintained by the Tokio team.
2. **Actix Web.** Very fast, mature, actor-influenced. Steeper learning curve; its own
   middleware model; historically more `unsafe` scrutiny.
3. **Rocket.** Ergonomic, macro-heavy; historically slower to track async/stable Rust.
4. **Custom framework.** Maximum control, enormous cost, contradicts "no unnecessary
   reinvention".

## Decision

Target **Axum** for generated apps in the MVP. Abstract the framework behind `gize-core`
conventions so that generated code depends on Axum directly (transparent, idiomatic) while
Gize's *generator* keeps a seam for future targets.

## Trade-offs

- (+) Gentle learning curve; idiomatic, readable generated handlers.
- (+) Tower ecosystem for middleware (tracing, timeout, auth layers).
- (+) First-class Tokio integration; strong community trajectory.
- (−) Not the absolute fastest in every benchmark (Actix often edges it), but the gap is
  small and rarely the bottleneck for CRUD apps.
- (−) Axum's extractor error ergonomics require careful DTO/error conventions (ADR-005).

## Consequences

- Generated handlers use Axum extractors and `IntoResponse`; middleware uses Tower layers.
- A future `gize-core` target for Actix is possible but explicitly out of scope until v2.
- The router registration convention (updating `router.rs`) is Axum-shaped.

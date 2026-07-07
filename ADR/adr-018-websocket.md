# ADR-018: Optional WebSocket scaffolding

- **Status:** Accepted
- **Date:** 2026-07-07
- **Deciders:** Gize core team

## Context

Realtime features (chat, live updates, notifications) are common in backend apps, and Axum has
first-class WebSocket support. Gize should let a project start with a working, typed WebSocket
endpoint without the developer wiring the upgrade handler, the message types and the route by
hand — but only when asked, so non-realtime projects stay lean.

## Decision

Add an opt-in flag to `gize new`:

```bash
gize new chat --ws
```

It scaffolds a self-contained module and records `features.websocket = true` in `gize.toml`.

**Structure — inside `src/app/`, not a top-level `src/websocket/`:**

```
src/app/ws/
  mod.rs       # declares submodules, re-exports `routes`
  message.rs   # typed ClientMessage / ServerMessage (serde, `type`-tagged)
  handler.rs   # upgrade handler + per-connection echo loop
  routes.rs    # Router fragment: GET /ws -> upgrade
```

`src/app/ws/` keeps the module under the existing app layout (ADR-005) and lets it be wired
through the **same registry marker** every other module uses (`mod ws;` +
`.merge(ws::routes())` in `app/mod.rs`). A top-level `src/websocket/` would need bespoke wiring
and break the "everything under `app/` is a module" convention.

**Routing — mounted through `app::routes()`, so it inherits the API prefix (ADR-016).** The
route is `/ws` on an unversioned project and `/api/v1/ws` on a versioned one, automatically —
no special-casing, because the ws router is merged into `app::routes()` like any module.

**Typed messages.** `message.rs` defines `type`-tagged enums (`ClientMessage`, `ServerMessage`)
so the protocol is explicit and extensible; the handler decodes with `serde_json` and echoes
`Echo { text }` back, replying `Error { message }` on a parse failure. `AppState` is threaded
into the upgrade handler (unused in the echo example) so auth/shared channels/DB are one step
away.

**Dependencies — added only with `--ws`.** Axum's WebSocket support is behind its `ws` feature,
and typed messages need `serde_json`; both are added to the generated `Cargo.toml` only when the
flag is used, keeping non-realtime projects' dependency set unchanged.

**Reconciliation.** `gize sync` regenerates the `ws` module drift-aware when
`features.websocket` is on, and re-wires it into `app/mod.rs` — the same treatment as the
OpenAPI module (it is wired but never a `[[module]]` resource, since it is not a model).

## Trade-offs

- (+) A working, typed realtime endpoint in one flag; nothing to wire by hand.
- (+) Reuses the module layout, registry wiring and API-prefix machinery — no new mechanism.
- (+) Zero cost when unused: no extra module, no extra dependencies.
- (−) The echo is a starting point, not a framework — no rooms/pub-sub/backpressure helpers yet
  (deliberately, to keep it readable and owned).
- (−) A second opt-in feature that `gize sync` must reconcile (small, mirrors OpenAPI).

## Consequences

- `gize-templates` gains a `ws` module (four file templates); `gize-core`'s `Features` gains
  `websocket`; `project::cargo_toml` takes a `websocket` flag for the conditional deps.
- `gize new --ws` scaffolds and wires the module; `gize sync` reconciles it when the feature is
  on.
- Acceptance: `gize new --ws` produces a project that compiles and, once served, accepts a
  WebSocket at `/ws` and echoes typed messages — verified end-to-end (a real client sent an
  `echo` and received it back, and a malformed frame got a typed `error`).

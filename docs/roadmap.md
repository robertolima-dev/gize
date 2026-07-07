# Gize — Roadmap (18 months)

> Time horizons are indicative and assume a small core team. Each phase lists objectives,
> dependencies, risks, features, and acceptance criteria. Phases are gated: a phase starts
> only when the previous one meets its acceptance criteria.

Legend for dependencies: ADRs are in `/ADR`, backlog items in `/BACKLOG.md`.

---

## Phase 1 — MVP (months 0–4)

**Status: Delivered (v0.5.x).**

**Objective:** Prove initial productivity. From `gize new` to a running CRUD API in
minutes, with readable generated code.

**Dependencies:** ADR-001 (workspace), ADR-002 (Axum), ADR-003 (SQLx), ADR-004
(templates), ADR-005 (module layout), ADR-009 (config), ADR-011 (migrations),
ADR-012 (CLI).

**Features:**
- `gize new`, `gize make app`, `gize make model`, `gize make crud`.
- `gize make migration`, `gize migrate` (Postgres only).
- `gize serve`, `gize doctor`.
- `gize.toml` parsing; safe file writer with `--force`/`--dry-run`.
- Snapshot + integration tests; base CI.

**Risks:**
- Codegen quality: generated code that isn't idiomatic kills trust. → snapshot tests + ADR-004.
- Scope creep into admin/auth. → hard OUT-of-scope list in `docs/mvp.md`.
- Compile-time UX (slow feedback). → keep generated crates lean.

**Acceptance criteria:** the Definition of Done in `docs/mvp.md` passes end-to-end in CI.

---

## Phase 2 — Alpha (months 4–8)

**Status: Delivered (v0.6.x).** Rich `gize.toml`, `gize sync`, migration diffing,
relationships, generated auth (Argon2 + JWT) and validation shipped.

**Objective:** Make Gize usable on a real side-project. Round out the developer loop and
introduce the manifest-driven workflow.

**Dependencies:** MVP complete; ADR-009 (config) hardened; ADR-008 (plugins) drafted.

**Features:**
- `gize sync` — reconcile app from `gize.toml` (idempotent, drift detection, dry-run).
- `gize make migration` diffing (model change → migration diff).
- `gize-auth` MVP: session/JWT scaffolding, password hashing, guards.
- `gize fmt`, `gize check` wrappers.
- Relationships in models (FKs, one-to-many) in codegen.
- Validation layer maturity (DTO ↔ error mapping).

**Risks:**
- `sync` destroying hand edits. → conservative reconciliation, mandatory dry-run preview.
- Auth security defaults. → security review before shipping.

**Acceptance criteria:** a reference app (blog or shop) can be built primarily through the
CLI + manifest, with auth-protected routes, and rebuilt from scratch reproducibly.

---

## Phase 3 — Beta (months 8–12)

**Status: Delivered (v0.7.0).** Admin, OpenAPI, SQLite (behind a dialect seam) and the plugin
API v0 shipped; the three acceptance criteria below are met and verified.

**Objective:** Feature-complete for the "productive CRUD backend" story. Introduce the
Admin and API docs.

**Dependencies:** Alpha complete; ADR-006 (admin), ADR-010 (OpenAPI).

**Features:**
- `gize-admin`: `gize make admin <Model>` → List/Create/Edit/Show/Delete, filters, search,
  pagination, auto-wired to backend.
- `gize-openapi`: spec generation from handlers/DTOs.
- Second database consideration (MySQL/SQLite) behind the data seam.
- Plugin API v0 (ADR-008): third-party generators/templates.
- Docs site scaffold (`website/`).

**Risks:**
- Admin surface area (frontend build, versioning). → treat admin as a separable crate.
- OpenAPI drift vs. actual routes. → generate from the same source of truth.

**Acceptance criteria:** admin CRUD works against a generated resource; OpenAPI spec
validates and matches routes; at least one external plugin builds against the API.

---

## Phase 4 — RC (months 12–15)

**Status: In progress (v0.8.x).** Entered at v0.8.0: the 1.0 feature set is **frozen** (new
frameworks such as Actix move to v2.0) and the project is hardening the existing surface.

**Objective:** Stabilize. Freeze public APIs and generated-code contracts. Harden.

**Dependencies:** Beta complete.

**Features:**
- [x] API/codegen stability guarantees and a deprecation policy — see [`STABILITY.md`](../STABILITY.md).
- [x] Generated-code contract enforced in CI: apps across every dialect + feature are
      rustfmt-clean and type-check (ADR-020).
- [ ] Upgrade/migration guide between Gize versions (regen-safe templates).
- [ ] Performance benchmarks + regression gates in CI.
- [x] Security review (auth, generated SQL, admin) — see [`SECURITY.md`](../SECURITY.md).
- [ ] Complete docs: Getting Started, Architecture, Cookbook, FAQ, Migration Guide.

**Risks:**
- Backward-compat lock-in too early. → mark unstable surfaces explicitly.

**Acceptance criteria:** no breaking changes planned for v1.0; docs complete; benchmarks
and security review pass; a non-core team ships an app on the RC.

---

## Phase 5 — v1.0 (months 15–18)

**Objective:** Production-ready 1.0 with a stability promise.

**Dependencies:** RC hardening complete.

**Features:**
- Semantic-versioning stability guarantee for CLI + generated-code contracts.
- Published crates on crates.io; installable CLI.
- Polished website, examples gallery, contribution guide.
- LTS-style support statement.

**Risks:**
- Ecosystem expectations exceeding capacity. → clear scope + roadmap for v2.

**Acceptance criteria:** 1.0 released; reproducible install; example apps in CI; public
docs; issue/triage process in place.

---

## Phase 6 — v2.0 (beyond 18 months)

**Objective:** Ecosystem expansion.

**Candidate features (subject to ADRs):**
- Alternative framework target (Actix) via `gize-core` seam.
- Additional databases as first-class (MySQL, SQLite).
- Background jobs & events (`jobs/`, `events/`) as first-class generators.
- gRPC / GraphQL surfaces.
- Marketplace of community plugins and templates.
- Managed/hosted admin option.

**Risks:** breadth diluting quality. → each expansion behind its own ADR and demand signal.

**Acceptance criteria:** defined per-feature at v2 planning; no v2 feature ships if it
compromises v1 stability guarantees.

---

## Cross-cutting tracks (all phases)

- **Quality:** unit + integration + snapshot tests, clippy `-D warnings`, rustfmt, Miri
  where applicable, benchmarks where meaningful.
- **Docs:** English-only, kept in lockstep with features.
- **ADRs:** every significant decision recorded before implementation.

# ADR-006: Admin interface (gize make admin)

- **Status:** Accepted
- **Date:** 2026-07-06
- **Deciders:** Gize core team

## Context

Gize generates a REST API with auth (ADR-013), relationships (ADR-014) and validation. The
Beta adds an **admin UI**: a CRUD dashboard over the generated resources
(List/Create/Edit/Show/Delete, plus search, filters and pagination). We need to decide the
stack and, crucially, **how the admin is built and served**, because that choice ripples into
every generated project's toolchain, deployment, and how "you own the code" holds up.

The admin is an **internal, authenticated client over an API that already exists** (the Axum
backend). It is not a public, SEO-sensitive, content site.

## Alternatives

1. **Server-rendered in Rust (Askama/Maud + HTMX).** No JS build; the admin compiles with the
   app. Purest "plain Rust you own", lightest toolchain. But richer table/grid UX (client-side
   sorting, complex filters, optimistic updates) is harder, and it couples admin markup to the
   backend crate.
2. **Separate React SPA (Vite + TypeScript).** A static single-page app that talks to the API
   with the JWT. Builds to static assets deployable anywhere (or served by the backend). The
   canonical "admin dashboard" pattern; rich data-grid ecosystem.
3. **Next.js app.** Full-stack React with SSR/SSG/Server Components/API routes. Powerful, but
   almost all of that value (SSR, SEO, API routes, RSC) is redundant against an existing Rust
   API and adds a Node runtime and framework "magic".
4. **`react-admin` framework.** Batteries-included admin (data provider → CRUD screens). Fast
   to stand up, but opinionated, pulls Material UI, and the developer owns less of the code.

## Decision

Generate a **separate React SPA with Vite + TypeScript** (option 2).

- **Why not server-rendered:** we want a genuinely rich data grid (sort/filter/paginate,
  inline edit) without reinventing it in server HTML, and we want the admin decoupled from the
  backend crate's build.
- **Why not Next.js:** the backend *is* the API. SSR/SEO/Server Components/API routes bring no
  value to an authenticated internal dashboard and add a Node runtime plus RSC complexity —
  the opposite of Gize's "no hidden runtime, transparent code" ethos. (Next remains the right
  tool for the public marketing site, which is a separate project.)
- **Why not react-admin:** generating plain, owned React components beats depending on a
  framework runtime, consistent with "you own the code."

**Generated stack (per resource, from the manifest):**

- **Vite + React + TypeScript** — static build, deployable independently.
- **TanStack Query** — server-state/data fetching against the API.
- **TanStack Table** — data grids (sort, filter, pagination).
- **React Hook Form + Zod** — forms and validation. The Zod schema mirrors the backend
  `validator` rules (email, min length, required), so both sides enforce the same contract.
- **Tailwind + shadcn/ui** — components.

**Generation:** `gize make admin <Model>` reads `gize.toml` and emits, per resource,
`List` / `Create` / `Edit` / `Show` screens, a typed API client, and the Zod schema derived
from the model's fields and relationships. `features.admin = true` records it in the manifest
so `gize sync` reconciles it.

**Serving/deployment:** the admin is a **separate artifact**. It builds to static files and is
deployed apart (any static host) or, optionally, served by the backend under `/admin`. Because
it is cross-origin by default, Gize generates a **CORS layer** on the backend (allowing the
admin origin) and the admin authenticates via the existing `POST /users/login` flow, storing
the JWT and sending it as a `Bearer` header. Admin routes are gated behind `is_admin`.

## Trade-offs

- (+) Right tool for an internal CRUD dashboard; mature grid/forms ecosystem; rich UX.
- (+) Static build deploys anywhere; admin lifecycle decoupled from the backend build.
- (+) Plain, owned React/TS — no framework runtime, no SSR magic.
- (+) One validation contract expressed twice (Rust `validator` ↔ Zod), generated together.
- (−) A second toolchain (Node/npm) enters the project — mitigated by keeping the admin a
  clearly separate, optional artifact.
- (−) Cross-origin means CORS + token handling to generate and get right (well-understood).
- (−) More codegen surface than server-rendered HTML.

## Consequences

- `gize-admin` owns the admin templates and the `make admin` generator; the admin is a
  **separable artifact** (its own directory/build), never coupling the backend's compile.
- The backend gains a generated CORS layer and admin-gated routes; `features.admin` in the
  manifest drives reconciliation via `gize sync`.
- OpenAPI (ADR-010) and the admin share the manifest as source of truth, keeping the API
  client and screens aligned with the backend.
- Acceptance (Beta): admin CRUD works in the browser against a generated resource, with
  auth, pagination and search.

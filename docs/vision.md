# Gize — Vision

> Gize is a backend application ecosystem for Rust, inspired by the productivity of
> Django but fully aligned with Rust's philosophy. The name comes from the plateau of
> the Great Pyramids — solid foundations meant to last.

## The problem

Rust is an excellent language for backend services: it is fast, safe, and predictable.
But **starting** a real-world backend in Rust is slow. Compared to Django, Rails or
Laravel, a Rust developer pays a high "day-one tax":

- There is no blessed project layout, so every team reinvents one.
- Wiring a router, database pool, migrations, config, auth and error handling is manual
  and repetitive.
- Generating a new resource (model → migration → repository → service → DTO → handler →
  routes → tests) is entirely by hand.
- There is no first-class admin, no scaffolding, no conventions.

The result: teams either avoid Rust for CRUD-heavy products, or they spend days on
boilerplate before writing a single line of business logic.

**Gize targets exactly this pain: initial productivity.** It should let a developer go
from `gize new shop` to a running, organized, production-shaped API in minutes — without
hiding the architecture and without "magic" code.

## Who it is for

- **Teams adopting Rust for web/backend** who want Django-like velocity without giving up
  Rust's guarantees.
- **Rust developers** tired of copy-pasting the same project skeleton.
- **Startups and product teams** that need CRUD APIs, an admin, and auth quickly, but also
  need the option to drop down to raw code when requirements get sharp.

## Design principles (non-negotiable)

- Zero-cost abstractions.
- Generated code is idiomatic and readable — the developer stays the owner of the code.
- No unnecessary dependencies; every dependency is justified in an ADR.
- Convention over configuration, but customization is always possible.
- Predictable architecture, explicit code, performance as a priority.
- Built for real projects, not just demos.

## What Gize is NOT

- **Not a Django clone.** We borrow the *experience* (conventions, scaffolding, admin,
  productivity), not the runtime model or the "magic".
- **Not a black box.** Generated code is committed to the user's repo and fully editable.
  There is no hidden framework runtime rewriting your handlers at runtime.
- **Not an ORM or a web framework of its own** (at least initially). Gize orchestrates
  best-in-class crates (Axum, SQLx, Tokio) behind clear conventions.

## Differentiators

1. **Transparent codegen.** Everything Gize generates is normal Rust you can read, diff,
   and edit. No runtime reflection, no proc-macro maze you can't escape.
2. **Batteries, but detachable.** Admin, auth, and OpenAPI are opt-in crates, not a
   monolith you must swallow.
3. **Manifest-driven, not manifest-locked.** `gize.toml` describes the app and powers
   `gize sync`, but hand edits are always respected.
4. **Rust-native ergonomics.** DTO validation, typed errors, and async-first design map to
   how idiomatic Axum apps are already written.

## Landscape comparison

### vs. Django (Python)
- **Django:** unmatched productivity, ORM, migrations, admin, huge ecosystem — but dynamic
  typing, GIL, slower runtime, "magic" (metaclasses, signals) that hides behavior.
- **Gize:** aims for the same productivity and conventions, with Rust's type safety,
  performance, and explicit generated code. We trade Django's runtime magic for
  compile-time codegen you can inspect.

### vs. plain Axum (Rust)
- **Axum:** excellent, minimal, unopinionated HTTP framework. Gives you routing and
  extractors and nothing else — layout, DB, migrations, auth, admin are all up to you.
- **Gize:** uses Axum underneath but adds conventions, scaffolding, and an ecosystem. You
  still get plain Axum code; Gize just writes the boring 80% for you.

### vs. Actix Web (Rust)
- **Actix:** very fast, mature, actor-influenced API; steeper learning curve, heavier
  ergonomics for newcomers.
- **Gize:** starts on Axum (see ADR-002) for a gentler, tower-based ecosystem, but the
  framework layer is abstracted (`gize-core`) so an Actix target is a future option.

### vs. Loco (Rust)
- **Loco:** the closest existing "Rails for Rust". Batteries-included, SeaORM-based,
  opinionated runtime.
- **Gize:** differs on two bets — (a) generated code transparency over a heavy framework
  runtime, and (b) SQLx-first over SeaORM (see ADR-003), keeping the developer closer to
  SQL and to idiomatic Axum. Loco is validation that the demand is real.

### vs. Rails (Ruby)
- **Rails:** the productivity gold standard — generators, migrations, convention over
  configuration, mature admin gems.
- **Gize:** same generator/convention DNA, compiled and type-safe. We accept slower "edit
  → run" cycles (compilation) in exchange for performance and safety, and mitigate with
  fast incremental builds and `gize serve` ergonomics.

### vs. Laravel (PHP)
- **Laravel:** artisan generators, Eloquent ORM, first-class ecosystem, great DX.
- **Gize:** the `gize make ...` commands are our "artisan", producing Rust instead of PHP.
  We give up Eloquent's dynamic convenience for explicit repositories and typed queries.

## Success criteria for the vision

Gize succeeds if a developer can:

1. Scaffold a new project and have it compile and serve in minutes.
2. Generate a full CRUD resource with one command and immediately hit its endpoints.
3. Read every generated file and understand it — no hidden behavior.
4. Delete Gize from the project and still have a working, idiomatic Rust codebase.

The last point is the ultimate test: **Gize is a productivity accelerator, not a cage.**

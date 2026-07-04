# Gize — MVP Definition

> The goal is **not** to rebuild Django. The goal is to solve the single biggest pain in
> the Rust backend ecosystem first: **initial productivity.**

## MVP thesis

A developer should be able to run a handful of commands and get a compiling, running,
production-shaped API with at least one full CRUD resource — all in idiomatic Rust they
can read and own.

## In scope (MVP)

The MVP is the CLI plus the codegen core needed to make these commands real:

| Command | Result |
| --- | --- |
| `gize new <project>` | Scaffold a Cargo project with the standard layout, config, DB pool, router, state, error handling — compiles and serves an empty app. |
| `gize make app <name>` | Create a module (`model, dto, repository, service, handler, routes, error, tests, mod`), register it in `app/mod.rs`, and wire its routes. |
| `gize make model <Name> field:Type ...` | Generate a model struct from field definitions. |
| `gize make crud <Name>` | Generate repository, service, DTO, handlers, routes, validation and tests for a resource. |
| `gize make migration` / `gize migrate` | Generate and apply SQL migrations for models. |
| `gize serve` | Run the generated app with hot-ish reload ergonomics. |
| `gize doctor` | Diagnose environment/project (toolchain, DB reachability, config sanity). |

Supporting pieces required by the above:

- `gize.toml` manifest parsing and validation.
- A safe file writer: never overwrite without confirmation; `--force` and `--dry-run`.
- Idempotent updates to "registry" files (`app/mod.rs`, router registration).
- Snapshot tests for generated output.

## Explicitly OUT of scope (MVP)

Deferred to Alpha/Beta so the MVP stays small:

- **Admin UI** (`gize make admin`) — the whole React/TS admin. Big surface, comes later.
- **`gize sync`** full manifest reconciliation — ships after generation is proven.
- **Auth** (`gize-auth`) beyond a placeholder.
- **OpenAPI** generation.
- **Plugins / extensibility** system.
- **Multiple database backends** — MVP targets **PostgreSQL** only.
- **Multiple web frameworks** — MVP targets **Axum** only (abstraction seam exists, second
  target does not).
- `gize fmt` / `gize check` / `gize sync` (thin wrappers can slip in but are not gating).

## Target stack for the MVP (see ADRs)

- Web: **Axum** (ADR-002)
- Data: **SQLx** + PostgreSQL (ADR-003)
- Migrations: **SQLx-based, SQL-first** (ADR-011)
- CLI: **clap** (ADR-012)
- Codegen: template-based, `askama` or `minijinja` (ADR-004)
- Runtime: **Tokio**

## Definition of Done for the MVP

The MVP is done when, on a clean machine with Rust + Postgres:

1. `gize new shop && cd shop && gize serve` yields a running server on a known port.
2. `gize make app products` produces a compiling module wired into the router.
3. `gize make crud Product name:String price:i32 active:bool` produces working
   `GET/POST/PUT/DELETE` endpoints backed by the database.
4. `gize make migration && gize migrate` creates and applies the `products` table.
5. Every generated file compiles with `cargo build`, passes `cargo clippy -D warnings`,
   and generated tests pass with `cargo test`.
6. Re-running a generator without `--force` never destroys hand edits.
7. All of the above is covered by integration + snapshot tests in Gize's own CI.

## Anti-goals for the MVP

- No hidden runtime framework — output is plain Axum/SQLx code.
- No feature added "because Django has it" without a concrete Rust user need.
- No premature abstraction: one web framework, one database, one ORM path.

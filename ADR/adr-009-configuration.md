# ADR-009: Configuration & the gize.toml manifest

- **Status:** Accepted (revised 2026-07-06 for the Alpha phase — see "Revision")
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Gize needs two kinds of configuration: (a) **project manifest** describing what the app is
made of (modules, features), which drives generation and `gize sync`; and (b) **runtime
config** for the generated app (DB URL, port, secrets). These must not be conflated.

## Decision

### Manifest — `gize.toml` (build/generation time)

Lives at the project root, describes the application shape:

```toml
[project]
name = "shop"

[stack]
framework = "axum"
database  = "postgres"
orm       = "sqlx"

[features]
authentication = true
admin          = true
openapi        = true

[modules]
list = ["users", "products", "orders"]
```

- Owned by Gize's CLI; it is the source of truth for `gize sync`.
- Hand edits are respected; `sync` reconciles missing scaffolding, never clobbers user code
  without `--force` and a dry-run preview.

### Runtime config — environment-first

Generated apps load runtime config from **environment variables** (12-factor), with typed
loading in `src/config/`, optionally layered with a local `.env` for development. Secrets
never live in `gize.toml`.

## Alternatives

- **One config file for everything.** Simpler but dangerously mixes build-time shape with
  runtime secrets; rejected.
- **YAML/JSON manifest.** TOML chosen for Cargo-ecosystem consistency and readability.
- **No manifest (pure convention from filesystem).** Loses a declarative single source for
  `sync` and for tooling to reason about the app.

## `gize sync` semantics (defined here, implemented in Alpha)

- Read `gize.toml`, compute the desired set of modules/features.
- Diff against the filesystem.
- For missing scaffolding: generate it.
- For drift (file exists but differs): **report**, never silently overwrite.
- Always support `--dry-run` (default preview) and require `--force` to overwrite.

## Trade-offs

- (+) Clear separation of concerns; declarative app shape; safe reconciliation.
- (+) TOML fits the Rust/Cargo mental model.
- (−) Two config surfaces to learn (manifest vs. runtime) — documented explicitly.
- (−) `sync` correctness is subtle; conservative defaults mitigate risk.

## Consequences

- `gize-core` defines the manifest schema + validation.
- `gize new` writes an initial `gize.toml`; `gize make ...` updates the `[modules]` list.
- Generated `src/config/` reads env vars; `.env.example` is generated for onboarding.

## Revision (Alpha) — the manifest becomes a rich, per-module source of truth

The MVP manifest stored modules as bare names (`[modules] list = ["users", "products"]`).
That is enough to record *presence*, but the Alpha acceptance criterion requires the app to
be **rebuilt from scratch reproducibly** from the manifest, and `gize sync` /
migration-diffing need each module's **fields** and **relationships**. Bare names cannot
carry that, and recovering it by parsing generated Rust is fragile and rejected.

The manifest therefore captures each module's shape declaratively:

```toml
[project]
name = "blog"

[[module]]
name = "users"
fields = ["name:String", "email:String", "password:String", "is_admin:bool"]

[[module]]
name = "posts"
fields = ["title:String", "body:String", "published:bool"]

  [[module.belongs_to]]
  target = "users"   # emits an author_id FK column -> users(id); see ADR-014
```

- **Fields** reuse the exact `name:Type` grammar the CLI already parses (`gize-core::field`),
  so the manifest and the command line share one definition of a model.
- **`belongs_to`** records 1-N relationships (ADR-014); the referenced module must also be in
  the manifest (validated on load).
- **Backward compatibility:** the loader still accepts the legacy `[modules] list = [...]`
  form (each name becomes a module with no fields) and rewrites into the new `[[module]]`
  form on the next write. A project generated before this revision keeps working; the first
  `gize make ...`/`gize sync` migrates its manifest.
- **`gize.toml` stays the single source of truth.** `gize new`/`make ...` write module shape
  into it; `gize sync` reads it to reconcile the tree; migration-diffing reads it to compute
  schema deltas. No sidecar/lock file — one file a human can read and edit.

This makes the manifest, not the generated source, authoritative for *what the app is*, which
is what "reproducible rebuild" demands.

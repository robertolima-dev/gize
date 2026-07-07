# Stability policy

This document defines what Gize promises to keep stable, what it does not, and how changes are
made. It takes effect with the **Release Candidate** phase (v0.8.0), when the feature set for
1.0 is **frozen** and the project shifts from adding features to hardening.

Gize follows [Semantic Versioning](https://semver.org). Pre-1.0 the surface below is frozen in
*intent*; the strict semver guarantee begins at **1.0.0**.

## What is stable (the product surface)

Gize's product is the CLI and the code it generates. The following are the covered surface:

1. **The `gize` CLI** — the command names, their observable behavior, and their flags:
   `new`, `make app|model|crud|migration|admin`, `createadmin`, `migrate`, `serve`, `sync`,
   `doctor`, `fmt`, `check`. Removing or renaming a command or flag, or changing its meaning, is
   a breaking change.
2. **The `gize.toml` schema** — the tables and fields Gize reads and writes (`[project]`,
   `[stack]`, `[features]`, `[api]`, `[[module]]`). A manifest valid under version *N* stays
   valid under later *N* releases; new optional fields may be added.
3. **The generated-code contract** — not byte-for-byte, but the guarantees a generated project
   relies on: it **compiles**, is **clippy-clean** and **rustfmt-clean**, follows the module
   layout ([ADR-005](./ADR/adr-005-module-layout.md)), exposes the documented routes and auth
   behavior ([ADR-013](./ADR/adr-013-auth.md)), and targets the selected database through the
   dialect seam ([ADR-015](./ADR/adr-015-second-database.md)). Regenerating with a compatible
   Gize version never destroys hand edits ([ADR-012](./ADR/adr-012-cli.md)).

## What is NOT stable

- **The exact bytes of generated code.** During RC, generated output may still be refined
  (formatting, comments, minor structure). The *contract* above holds; the bytes may change
  between minor versions. This is why `gize sync` is drift-aware and never overwrites hand edits
  without `--force`.
- **The plugin API** (`gize-generator`'s `Generator` / `GenContext`) — explicitly **v0** and
  unstable ([ADR-008](./ADR/adr-008-plugins.md)). It may change until a dedicated plugin-API
  stabilization.
- **The library crates' Rust APIs** (`gize-core`, `gize-generator`, `gize-templates`, `gize-db`,
  `gize-openapi`, `gize-admin`, `gize-macros`). They are published so the CLI and plugins can
  build, but they are implementation details, not a supported library surface. Depend on the
  `gize` CLI, not on these crates' internals.

## Supported toolchain (MSRV)

- **Rust 1.85+**, **edition 2024**. Raising the minimum supported Rust version is a minor-version
  change, documented in the changelog.
- Generated projects target the same edition and pin their dependencies in a committed
  `Cargo.lock`.

## Deprecation policy

- A change that removes or renames part of the **stable surface** (a command, a flag, a
  `gize.toml` field) is **deprecated first**: it keeps working, emits a notice where practical,
  and is documented in the changelog for at least one minor release before removal, which only
  happens in a **major** version.
- A change to the **generated-code contract** that requires users to regenerate or migrate ships
  with a migration note in the changelog (and, from 1.0, in the migration guide).
- Everything under "What is NOT stable" may change in any release; such changes are still noted
  in the changelog.

## Feature freeze (1.0 line)

As of v0.8.0 the 1.0 feature set is **frozen**. RC releases (0.8.x) harden the existing surface —
security review, benchmarks, migration/regeneration safety, and documentation — rather than add
features. New capabilities that would change the stable surface (for example an **Actix Web**
backend) are planned for **v2.0** ([roadmap](./docs/roadmap.md)).

## How to report a stability concern

If a Gize upgrade breaks a project in a way not described here, that is a bug — please open an
issue with the versions and a minimal reproduction.

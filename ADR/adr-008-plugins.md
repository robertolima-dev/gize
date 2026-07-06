# ADR-008: Plugin API (extensibility, v0)

- **Status:** Accepted
- **Date:** 2026-07-06
- **Deciders:** Gize core team

## Context

Gize's generators (project, module, crud, admin, openapi) are built in. The Beta introduces a
**plugin API v0** so third parties can add their own generators/templates — e.g. a
`healthcheck` scaffolder, a custom resource layout, an alternative admin — without forking
Gize. The goal is a *minimal, honest* extension seam, explicitly marked unstable, not a
full marketplace (that is a v2 concern).

## Alternatives

1. **In-process trait, statically linked (Rust plugins as crates).** A plugin is a Rust crate
   implementing a `Generator` trait; it is compiled into a `gize` build (or a wrapper binary)
   and discovered via a registry. Type-safe, no dynamic loading, no ABI concerns. The plugin
   produces a `Plan` (the same safe file-op type the built-in generators use).
2. **Dynamic loading (`dylib`/`abi_stable`).** Load `.so`/`.dylib` plugins at runtime. Powerful
   but Rust has no stable ABI; `abi_stable` is heavy and fragile. Rejected for v0.
3. **Process/CLI convention (`gize-<name>` binaries on PATH), à la git/cargo.** A plugin is any
   executable named `gize-<name>`; `gize <name> …` shells out to it. Language-agnostic, fully
   decoupled, no ABI. But data exchange is stringly-typed and the plugin can't reuse Gize's
   `Plan`/`Writer` safety directly.
4. **WASM plugins.** Sandboxed and portable, but a large runtime and a serialization boundary;
   premature for v0.

## Decision

For **v0**, adopt the **in-process `Generator` trait** (option 1) as the primary API, and
reserve the **`gize-<name>` subcommand convention** (option 3) as the escape hatch for
out-of-tree/other-language tools.

- A plugin implements a small trait exposed by `gize-generator`, roughly:

  ```rust
  pub trait Generator {
      /// Subcommand name, e.g. "healthcheck".
      fn name(&self) -> &str;
      /// Build a Plan from the manifest + args; no I/O (stays testable, honors --dry-run).
      fn plan(&self, ctx: &GenContext, args: &Args) -> anyhow::Result<Plan>;
  }
  ```

- Plugins return a **`Plan`**, so every third-party generator inherits the safety model
  (never clobber without `--force`, `--dry-run`, drift-aware via `gize sync`) and the
  manifest as source of truth. No plugin writes files directly.
- Discovery for v0 is an explicit **registry** (plugins registered in a build), plus the
  `gize-<name>` PATH fallback for tools that can't link in.
- The API is **marked `v0`/unstable**: the trait and `GenContext` may change between minor
  versions until stabilized in RC.

## Trade-offs

- (+) Type-safe, reuses `Plan`/`Writer`/manifest — plugins are as safe as built-ins.
- (+) No ABI/dynamic-loading fragility; simple mental model.
- (+) The `gize-<name>` fallback keeps the door open for non-Rust tools.
- (−) In-process plugins require a build that includes them (no drop-in `.so`). Acceptable for
  v0; dynamic/WASM loading can come later behind demand.
- (−) A published-but-unstable API risks churn — mitigated by the explicit `v0` label and a
  narrow surface.

## Consequences

- `gize-generator` defines the `Generator` trait and `GenContext` (manifest access, naming,
  timestamp); the CLI dispatches unknown subcommands to registered plugins, then to
  `gize-<name>` on PATH.
- Plugin output flows through the same `Plan` → `Writer`/`sync` path as everything else.
- Stabilization of the trait is deferred to the RC (with a deprecation policy).
- Acceptance (Beta): at least one **external** plugin builds against the v0 API and generates
  through the safe writer.

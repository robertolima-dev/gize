# ADR-020: Format generated code with rustfmt

- **Status:** Accepted
- **Date:** 2026-07-07
- **Deciders:** Gize core team

## Context

Gize's generated Rust compiled and passed clippy, but it was **not rustfmt-clean**: running
`cargo fmt --check` on a fresh `gize new` project reported dozens of diffs (import ordering,
method-chain indentation, line wrapping, trailing-comment alignment, `mod` ordering in
`app/mod.rs`). That is a real rough edge — a user whose CI runs `cargo fmt --check`, or who runs
`gize check`, sees churn on code they never wrote — and it undermines the "clean, reviewable
generated code" promise on the road to a stable release.

Making the **templates** match rustfmt by hand is not enough: rustfmt's output depends on line
widths, which depend on model and field names (e.g. a longer model name wraps a signature that a
shorter one keeps on one line; the `app/mod.rs` merge chain collapses onto one line for few
modules and wraps for many). No static template can be rustfmt-idempotent for *all* inputs.

## Decision

**Run `rustfmt` over the generated `.rs` files at write time.** This makes output match the
toolchain's formatting exactly, for any names, and stays correct as rustfmt evolves.

- The `Writer` gained an `Options.format` flag. When set, after writing a plan it runs
  `rustfmt --edition 2024` over the `.rs` files it just created/overwrote. The CLI turns it on
  (`From<GenFlags>`); tests and plugins using `Options::default()` leave it off.
- `app/mod.rs` is edited in place by the registry (not through the `Writer`), so the CLI runs
  rustfmt over it after each registration — this sorts the `mod` declarations and
  collapses/wraps the merge chain, which a marker-based text insertion cannot do.
- `gize sync` formats the files it reconciles the same way.
- It is **best-effort**: if `rustfmt` is missing or fails, the files are left as written (valid
  Rust, just unformatted) — generation never fails because of formatting.

**Templates and their snapshots stay the raw, reviewed source.** Snapshot tests read the
in-memory `Plan` (the pre-write template output), so they remain deterministic and independent
of the installed rustfmt version. Formatting is applied only on the way to disk. A CI job
(`generated`) generates apps across all dialects and features and asserts each is
`cargo fmt --check`-clean and type-checks, so the on-disk guarantee is enforced.

## Trade-offs

- (+) Generated projects are rustfmt-clean for any inputs; `gize check` / `cargo fmt --check`
  pass out of the box.
- (+) Templates and snapshots stay raw and deterministic (no rustfmt-version coupling in tests).
- (+) Future templates are formatted automatically — no per-template fmt maintenance.
- (−) Generation now shells out to `rustfmt` (already an expected toolchain component; checked by
  `gize doctor`). Mitigated by the graceful fallback.
- (−) The on-disk output differs slightly from the raw snapshot (formatting), so the snapshot is
  the template source of truth, not a byte-for-byte image of disk — documented.
- (−) `app/mod.rs` is normalized by rustfmt on each `make`/`sync`; acceptable since that file is
  generated wiring the command already edits.

## Consequences

- `gize-generator` exposes `format_rust_files`; the `Writer` formats on write when
  `Options.format` is set; `gize sync` and the `app/mod.rs` registry edit format their output.
- A `generated` CI job enforces that generated apps are rustfmt-clean and type-check across
  postgres, sqlite and mysql (with OpenAPI, WebSocket, API versioning and a CRUD resource) — a
  regression net for the generated-code contract on the way to RC.

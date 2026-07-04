# ADR-004: Code generation strategy (templates)

- **Status:** Accepted
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

Gize's core value is generating idiomatic Rust files. We must choose *how* code is
generated: string templates, a typed AST builder, or proc-macros. The output must be
readable, formatted, and stable across runs (for snapshot testing).

## Alternatives

1. **Text templates** (`askama`, `minijinja`, `tera`, `handlebars`).
   - `askama`: compile-time-checked Jinja-like templates → type-safe, fast, but templates
     are baked into the binary and less runtime-flexible.
   - `minijinja`: runtime Jinja engine → flexible, easy to load templates from
     `gize-templates`, great error messages.
2. **Typed code builder** (`quote`/`syn`, `prettyplease`). Build a token stream / AST and
   pretty-print. Very robust, but verbose to author and awkward for large file skeletons.
3. **Proc-macros at the user's compile time.** Generate at build time in the user's crate.
   Rejected as the primary mechanism: it hides code (against the philosophy) — see ADR-007.

## Decision

Use **text templates via `minijinja`**, with templates living in `gize-templates` and
rendered by `gize-generator`. Generated Rust is **always run through `rustfmt`** (or
`prettyplease` where a token stream is used) before writing, so output is canonical and
snapshot-stable.

For a few surgical, structural edits (e.g. inserting a module into `app/mod.rs`, wiring a
route into `router.rs`), use **`syn`-based AST edits** rather than fragile string
insertion, so idempotency and correctness are guaranteed.

## Trade-offs

- (+) `minijinja` keeps templates editable and separable from the binary; good errors.
- (+) `rustfmt` post-processing guarantees idiomatic, diff-friendly output.
- (+) `syn` edits for registry files are robust and idempotent.
- (−) Two mechanisms (templates + AST edits) to maintain.
- (−) Runtime templates lose compile-time template checking (mitigated by snapshot tests).

## Consequences

- `gize-generator` exposes: `render(template, context) -> String` then `format_rust()` then
  `write_safely()` (respecting `--force`/`--dry-run`).
- Registry mutations (`mod.rs`, `router.rs`) go through a dedicated `syn`-based editor that
  is a no-op if the entry already exists.
- Every generator has a golden snapshot test.

## Update — 2026-07-04 (MVP implementation note)

The MVP ships two deliberate simplifications of the above, both behind stable
pure-function boundaries so the target design can slot in without API churn:

1. **Templates are Rust functions**, not `minijinja` files yet (`gize-templates`). The
   migration to on-disk `minijinja` templates is a follow-up; callers already go through
   functions returning `String`.
2. **Registry edits are marker-based**, not `syn`-based yet (`gize-generator::registry`).
   The generated `app/mod.rs` carries `// gize:modules` and `// gize:module-routes`
   markers; insertion is idempotent (a no-op if the module is already registered) and
   covered by unit tests. A `syn`-based editor remains the hardening target for robustness
   against hand edits that move the markers.

`rustfmt` post-processing of generated Rust is also deferred; current templates are
authored pre-formatted and verified by compiling a generated project in the e2e check.

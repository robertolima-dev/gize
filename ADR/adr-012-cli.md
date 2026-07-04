# ADR-012: CLI design

- **Status:** Accepted
- **Date:** 2026-07-04
- **Deciders:** Gize core team

## Context

The CLI is Gize's primary interface. Command structure, argument syntax, and safety
behavior define the whole developer experience. The prompt lists candidate commands; we
must decide the surface, the parser, and the safety model.

## Decision

### Parser
Use **`clap` (derive API)** — the de facto standard: subcommands, help, validation,
completions. Rejected alternatives: hand-rolled parsing (reinvention), `argh`/`pico-args`
(less featureful for a rich subcommand tree).

### Command surface (MVP + planned)

```
gize new <project>                         # scaffold a project
gize make app <name>                       # scaffold a module
gize make model <Name> field:Type ...      # generate a model
gize make crud <Name>                      # generate full CRUD
gize make migration [name]                 # generate a migration
gize make admin <Name>                     # (Beta) admin UI for a model
gize migrate [--status]                    # apply migrations
gize serve                                 # run the app
gize sync                                  # (Alpha) reconcile from gize.toml
gize doctor                                # diagnose environment/project
gize fmt                                   # wrapper around rustfmt
gize check                                 # wrapper around clippy/check
```

`make` is a subcommand group (`make app|model|crud|migration|admin`) to keep the noun/verb
model consistent and discoverable.

### Model field syntax
Support **inline** form as primary: `gize make model User name:String email:String
active:bool age:i32`. This is scriptable, copy-pasteable, and matches Rails/Laravel muscle
memory. A future interactive/multiline prompt may be added, but inline is the canonical UX
(supersedes the open question in the backlog).

### Safety model (applies to all generators)
- **Never overwrite without confirmation.** Existing files are left untouched unless
  `--force` is passed.
- `--dry-run` prints the planned file operations (create/update/skip) and writes nothing.
- Registry edits (`app/mod.rs`, `router.rs`) are **idempotent** — re-running is a no-op if
  the entry exists.
- Every mutating command prints a clear summary of what it did.

### Is the command set sufficient?
For the MVP, yes. Gaps identified for later phases (not MVP-blocking): `gize generate
completions`, `gize new --template <name>`, `gize db reset/seed`, `gize routes` (list
routes). These are logged to the backlog rather than committed now.

## Trade-offs

- (+) `clap` gives help/validation/completions for free; noun-verb structure scales.
- (+) Inline model syntax is scriptable and familiar.
- (−) `clap` derive adds compile time to `gize-cli` (acceptable; it's the CLI, not user code).

## Consequences

- `gize-cli` defines the clap command tree and delegates to `gize-generator`/`gize-db`.
- The safety flags (`--force`, `--dry-run`) are shared options implemented in the generator,
  not per-command.
- Backlog UX question on model syntax is resolved: **inline is canonical.**

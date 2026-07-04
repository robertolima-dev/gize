# gize-generator

**Code generation engine for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![Crates.io](https://img.shields.io/crates/v/gize-generator.svg)](https://crates.io/crates/gize-generator)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-generator` turns [`gize-core`](https://crates.io/crates/gize-core) specs into files —
**safely**. It has two responsibilities:

1. Render file contents from model/project specs (via
   [`gize-templates`](https://crates.io/crates/gize-templates)).
2. Write those files without ever clobbering user code, honoring `--force` and `--dry-run`
   (see [ADR-012](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-012-cli.md)).

## Key types

- **`Plan` / `FileOp` / `OpKind`** — a pure, inspectable description of what will be written
  (`create` / `skip` / `update`) before anything touches disk.
- **`Writer` / `Options` / `Report`** — executes a `Plan`, applying the safety model and
  returning a report of what happened.
- **`register_module` / `Edit`** — idempotent registry edits that wire new modules into
  `app/mod.rs` and `gize.toml` without disturbing surrounding code.

## The safety model

- Existing files are **skipped** unless `--force` is passed.
- `--dry-run` produces a full `Plan` and writes nothing.
- Registry edits are **idempotent** — re-running a generator never duplicates entries.

## Usage

```toml
[dependencies]
gize-generator = "0.2"
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model & conventions |
| **`gize-generator`** | Codegen engine (this crate) |
| `gize-templates` | Templates for generated code |
| `gize-db` | Data-layer conventions + migrations |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

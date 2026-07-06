# gize-generator

**Code generation engine for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![Crates.io](https://img.shields.io/crates/v/gize-generator.svg)](https://crates.io/crates/gize-generator)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-generator` turns [`gize-core`](https://crates.io/crates/gize-core) specs into files,
**safely**. It has three responsibilities:

1. Render file contents from model/project specs (via
   [`gize-templates`](https://crates.io/crates/gize-templates),
   [`gize-openapi`](https://crates.io/crates/gize-openapi) and
   [`gize-admin`](https://crates.io/crates/gize-admin)).
2. Write those files without ever clobbering user code, honoring `--force` and `--dry-run`
   (see [ADR-012](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-012-cli.md)).
3. Reconcile a project from its manifest (`gize sync`) and expose a small plugin API so third
   parties can add their own generators.

## Key types

- **`Plan`, `FileOp`, `OpKind`**: a pure, inspectable description of what will be written
  (create / skip / update) before anything touches disk.
- **`Writer`, `Options`, `Report`**: executes a `Plan`, applying the safety model and
  returning a report of what happened.
- **`sync`**: reconciles the tree against the manifest, classifying each file as missing,
  drifted or unchanged, and creating only what is missing unless `--force` is given.
- **`register_module`, `Edit`**: idempotent registry edits that wire new modules into
  `app/mod.rs` without disturbing surrounding code.
- **`Generator`, `GenContext`** (`plugin`): the v0 plugin API. A plugin returns a `Plan`, so
  it inherits the same safe writer for free (see
  [ADR-008](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-008-plugins.md)).

## The safety model

- Existing files are **skipped** unless `--force` is passed.
- `--dry-run` produces a full `Plan` and writes nothing.
- Registry edits and `sync` are **idempotent**: re-running never duplicates or clobbers.

## Usage

```toml
[dependencies]
gize-generator = "0.7"
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model, manifest, dialect, conventions |
| **`gize-generator`** | Codegen engine: safe writer, sync, plugins (this crate) |
| `gize-templates` | Templates for the generated code |
| `gize-db` | Migrations (PostgreSQL and SQLite) |
| `gize-openapi` | OpenAPI spec generation |
| `gize-admin` | Admin UI generator |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

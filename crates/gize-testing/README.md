# gize-testing

**Test utilities for [Gize](https://github.com/robertolima-dev/gize)-generated applications.**

[![Crates.io](https://img.shields.io/crates/v/gize-testing.svg)](https://crates.io/crates/gize-testing)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-testing` provides small, dependency-free helpers to exercise a Gize-generated app end to
end, so the automated "compile, serve, call routes" loop is easy to run (including in CI).

- **`EphemeralSqlite`**: a throwaway SQLite database (serverless, no database process needed),
  removed on drop. Ideal for CI runners.
- **`App`**: spawns a compiled app binary against an `EphemeralSqlite`, waits until it accepts
  connections, and exposes `base_url()`. The child process is killed on drop.
- **`free_port`**: pick a free TCP port.

Bring your own HTTP client for the assertions.

## Usage

```toml
[dev-dependencies]
gize-testing = "0.7"
```

```rust
use gize_testing::{App, EphemeralSqlite};

let db = EphemeralSqlite::new();
// (apply migrations to db.url() first)
let app = App::spawn("target/debug/blog", &db)?;
// app.base_url() is now serving; make requests, assert, then drop to shut down.
# Ok::<(), std::io::Error>(())
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model, manifest, dialect, conventions |
| `gize-generator` | Codegen engine: safe writer, sync, plugins |
| `gize-templates` | Templates for the generated code |
| **`gize-testing`** | Test utilities for generated apps (this crate) |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

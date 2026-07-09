# gize-openapi

**OpenAPI spec generation for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![Crates.io](https://img.shields.io/crates/v/gize-openapi.svg)](https://crates.io/crates/gize-openapi)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-openapi` generates an OpenAPI 3.0.3 spec from the **manifest** (`gize.toml`) plus the
DTOs, the same source of truth the CRUD and admin generators use, so the spec matches the
generated routes by construction rather than by hand-kept annotations. See
[ADR-010](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-010-openapi.md).

The spec covers each resource's CRUD plus the `users` register/login and self-service
`GET /users/me` endpoints, marks write and admin-gated routes as bearer-secured, hides
`password` from responses, and includes relationship foreign keys. Enable it with `gize new --openapi` (or `features.openapi` in `gize.toml`); the app then
serves it at `GET /openapi.json` with a reference UI at `/docs`, and `gize sync` keeps it in
step with the manifest.

## Usage

```toml
[dependencies]
gize-openapi = "0.7"
```

```rust
let manifest = gize_core::Manifest::from_toml(toml_text)?;
let spec = gize_openapi::spec_json(&manifest)?; // serde_json::Value
# Ok::<(), anyhow::Error>(())
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model, manifest, dialect, conventions |
| `gize-generator` | Codegen engine: safe writer, sync, plugins |
| **`gize-openapi`** | OpenAPI spec generation (this crate) |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

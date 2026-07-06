# gize-admin

**Admin UI generator for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![Crates.io](https://img.shields.io/crates/v/gize-admin.svg)](https://crates.io/crates/gize-admin)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

`gize-admin` generates a **separate** Vite + React + TypeScript admin SPA for your resources,
exposed through `gize make admin` (see
[ADR-006](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-006-admin.md)).

The admin is **data-driven from the manifest**: it emits one descriptor per resource (fields
plus a Zod schema that mirrors the backend validation), and a single generic `Resource`
component renders List, Create, Edit and Delete for any of them, with search and pagination.
It authenticates with the existing JWT login and reaches the API through a Vite dev proxy, so
the backend needs no CORS or other changes. The generated app lives under `admin/` and builds
independently (`npm install && npm run dev`).

Stack: Vite, React, TypeScript, TanStack Query and Table, React Hook Form, Zod, Tailwind.

## Usage

This crate provides the template functions consumed by
[`gize-generator`](https://crates.io/crates/gize-generator). As a user you invoke it through
the CLI:

```bash
gize make admin
cd admin && npm install && npm run dev
```

## Part of the Gize workspace

| Crate | Role |
| --- | --- |
| `gize-core` | Domain model, manifest, dialect, conventions |
| `gize-generator` | Codegen engine: safe writer, sync, plugins |
| `gize-templates` | Templates for the generated code |
| **`gize-admin`** | Admin UI generator (this crate) |
| `gize` | The `gize` CLI |

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

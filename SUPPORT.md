# Support

## Getting help

1. Read the [FAQ](./docs/faq.md), [Getting started](./docs/getting-started.md) and
   [Cookbook](./docs/cookbook.md). Most questions are answered there.
2. Run `gize doctor` inside your project; it checks the toolchain, `.env`, `DATABASE_URL` and
   `GIZE_JWT_SECRET`.
3. Search the [issues](https://github.com/robertolima-dev/gize/issues). If nothing matches, open
   one with the Gize version (`gize --version`), your database, and a minimal reproduction.
   [`CONTRIBUTING.md`](./CONTRIBUTING.md#issue-triage) explains what happens next.

Report security vulnerabilities privately, as described in [`SECURITY.md`](./SECURITY.md).

## Supported versions

| Version | Bug fixes | Security fixes |
| --- | --- | --- |
| Latest 1.x minor | Yes | Yes |
| Previous 1.x minor | No | For 3 months after the next minor is released |
| Older 1.x minors | No | No |
| 0.x (pre-1.0) | Latest 0.9.x only, until 1.0.0 | Latest 0.9.x only, until 1.0.0 |

Fixes ship as patch releases (`1.y.z`). Within 1.x, upgrading to the latest minor is meant to be
safe: the CLI, the `gize.toml` schema and the generated-code contract follow the Semantic
Versioning promise in [`STABILITY.md`](./STABILITY.md), and [`MIGRATION.md`](./MIGRATION.md)
lists anything to review when you regenerate code.

Install or upgrade with `cargo install gize --locked`. Gize supports Rust **1.85+**; raising that
minimum is a minor-version change announced in the changelog.

## What is supported

- The `gize` CLI and the code it generates, on PostgreSQL, MySQL and SQLite.
- The documented commands, flags and `gize.toml` fields.

Not supported: the Rust APIs of the library crates (`gize-core`, `gize-generator`, …), which are
implementation details, and the plugin API, which is v0 and may change (see `STABILITY.md`).
Generated code you have edited is yours; we help with what Gize generated, not with changes made
afterwards.

# Changelog

All notable changes to Gize are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(pre-1.0: minor versions may introduce changes to generated output).

## [0.4.0] - 2026-07-05

### Added

- `gize new` now scaffolds a built-in `users` resource by default: model, full CRUD
  (dto, repository, service, handler, routes, error, tests) and a migration, wired into
  `src/app/mod.rs` and `gize.toml` automatically.
  - Minimal, authentication-ready fields: `id`, `name`, `email`, `password`, `is_admin`,
    plus `created_at` / `updated_at`.
  - `is_admin` (`BOOLEAN NOT NULL DEFAULT false`) is included from day one as the flag a
    future admin panel / `gize-auth` can gate access on.
  - `email` is `UNIQUE`; `password` is `#[serde(skip_serializing)]`, so its hash never
    leaks into API responses.
- `gize new --no-user` opts out of the built-in `users` resource and scaffolds the bare
  project skeleton.
- `Plan::extend` in `gize-generator` to compose a base plan with an optional add-on slice.

### Notes

- The generated project compiles and passes `cargo clippy -D warnings` end to end.
- Follow-ups tracked in `BACKLOG.md`: password hashing on create/update, removing
  `is_admin` from the `CreateUser` DTO, and register/login endpoints — all pending the
  `gize-auth` work.

[0.4.0]: https://github.com/robertolima-dev/gize/releases/tag/v0.4.0

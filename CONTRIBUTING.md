# Contributing to Gize

Thanks for helping. This guide covers how to build and test Gize, how changes get decided and
merged, and what happens to an issue after you open it.

## Before you start

- **Questions** go to the [FAQ](./docs/faq.md), [Getting started](./docs/getting-started.md) and
  [Cookbook](./docs/cookbook.md) first; [`SUPPORT.md`](./SUPPORT.md) says where to ask next.
- **Security issues** must not be opened as public issues. Follow [`SECURITY.md`](./SECURITY.md).
- **Architectural changes** need an ADR first. Gize records every significant decision in
  [`ADR/`](./ADR) before implementing it ([ADR-000](./ADR/adr-000-process.md) describes the
  process). Anything that changes the stable surface in [`STABILITY.md`](./STABILITY.md) (CLI
  commands and flags, the `gize.toml` schema, the generated-code contract) needs one.
- **New features** are frozen for the 1.x line: the 1.0 feature set was frozen at 0.8.0. Fixes,
  documentation and hardening are welcome; feature ideas are welcome as issues and are
  collected for v2.0 ([roadmap](./docs/roadmap.md)).

## Development setup

You need Rust **1.85+** (the MSRV) with `rustfmt` and `clippy`. The end-to-end script also needs
`python3` and `curl`, plus a PostgreSQL or MySQL server if you test those databases.

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

### Testing generated apps

Most Gize changes are changes to the code it **generates**, so unit tests are not enough. Run the
end-to-end script, which generates an app, runs it against a real database, calls every route,
then evolves the schema and calls them again:

```bash
cargo build -p gize
GIZE=target/debug/gize DATABASE=sqlite scripts/e2e.sh
GIZE=target/debug/gize DATABASE=postgres DATABASE_URL=postgres://user:pass@localhost/empty_db scripts/e2e.sh
GIZE=target/debug/gize DATABASE=mysql DATABASE_URL=mysql://user:pass@127.0.0.1/empty_db scripts/e2e.sh
```

CI runs it on SQLite, PostgreSQL 14 and 16, and MySQL 5.7 and 8.0, and checks that the workspace
and freshly generated apps build on Rust 1.85.

### Snapshot tests

`crates/gize-generator/tests/snapshots/` holds the exact output of the generators. When you change
a template on purpose, regenerate them and review the diff before committing:

```bash
UPDATE_SNAPSHOTS=1 cargo test -p gize-generator --test generation
git diff crates/gize-generator/tests/snapshots
```

## Pull requests

- Keep a PR to one change. Explain what it changes for someone using `gize` or a generated app.
- `cargo fmt`, `cargo clippy -D warnings` and `cargo test --workspace` must pass. If you touched
  templates, run `scripts/e2e.sh` on at least SQLite.
- Add tests: a template test for generated text, a CLI test (`crates/gize/tests/`) for command
  behavior, or an `e2e.sh` check for runtime behavior.
- Add an entry to [`CHANGELOG.md`](./CHANGELOG.md). If generated code changes in a way users must
  act on, add a note to [`MIGRATION.md`](./MIGRATION.md).
- Contributions are licensed under MIT OR Apache-2.0, like the project.

## Issue triage

Every new issue gets a first response and a label within **7 days**. This is a goal, not a
guarantee; Gize is maintained on a best-effort basis. Security reports follow `SECURITY.md`.

| Label | Meaning |
| --- | --- |
| `bug` | Gize or the code it generates does something wrong. |
| `needs-repro` | We could not reproduce it yet; a minimal reproduction moves it forward. |
| `enhancement` | A new capability. Collected for v2.0 unless it is a fix in disguise. |
| `documentation` | The documentation is wrong, missing or unclear. |
| `question` | Answered and closed, or turned into a `documentation` issue. |
| `rc-validation` | A report from running [`docs/validation.md`](./docs/validation.md). |
| `good first issue` | Small and well scoped; a good starting point. |

Priority, from highest:

1. Security issues (handled privately).
2. A generated app that does not compile, fails at runtime, or loses data.
3. A broken documented command or install path.
4. Wrong or confusing documentation and error messages.
5. Enhancements.

Issues waiting on the reporter for 30 days are closed; they can be reopened with the requested
information.

## Releases

Maintainers release from `main`: the version is bumped in the workspace `Cargo.toml`, the
changelog section is dated, and a `vX.Y.Z` tag triggers `release.yml`, which reruns the full CI
and publishes the crates to crates.io in dependency order.

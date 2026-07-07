# ADR-019: `gize serve` boots the admin dev server

- **Status:** Accepted
- **Date:** 2026-07-07
- **Deciders:** Gize core team

## Context

`gize serve` ran only the API (`cargo run`). When a project also has a generated admin (ADR-006,
a separate Vite + React app under `admin/`), the developer had to open a second terminal, `cd
admin`, install dependencies and run the dev server by hand. For a productivity-first framework,
one command should bring up the whole dev environment.

## Decision

`gize serve` gains admin awareness.

**Default:** run the API, and — when an admin has been generated (`features.admin` and an
`admin/` directory) — the admin dev server alongside it. A project without an admin behaves
exactly as before (API only), so nothing regresses.

**Flags (mutually exclusive):**

- `--api-only` — run only the API.
- `--admin-only` — run only the admin dev server.
- `--with-admin` — the explicit form of the default (run both).

Requesting the admin (`--admin-only` / `--with-admin`) when none exists fails with guidance to
run `gize make admin`.

**No new manifest config.** The decision keys off the existing `features.admin`; a separate
`[admin]` table with a duplicate `enabled` flag was rejected as redundant. The admin dev-server
port is Vite's default (5173) and is not re-declared in `gize.toml` — the dev server prints its
own exact URL.

**Package manager:** detected on `PATH` in the order **pnpm → npm → yarn** (first found). On the
first run, if `admin/node_modules` is missing, `<pm> install` runs before `<pm> run dev`.

**Process model.** In "both" mode the API (`cargo run`) and the admin (`<pm> run dev`) are
spawned as children with inherited stdio, and a small supervisor loop waits on both: when either
exits, the other is stopped, so a crash of one side does not leave the other running. An
interactive **Ctrl-C** is delivered by the terminal to the whole foreground process group, so
both children (and their `cargo`/`vite` subprocesses) receive `SIGINT` and shut down together —
the supervisor then reaps them. Verified: `gize serve` brought up the API (`:8080`) and the admin
(`:5173`) together via `npm install` + `npm run dev`, and tearing the tree down left no orphaned
processes and no ports held.

## Trade-offs

- (+) One command boots the full dev environment; zero extra config.
- (+) No dependency on `features.admin` duplication; no new manifest surface.
- (+) No orphaned processes on the normal Ctrl-C path (terminal group signal) or on a
  sibling-exit (supervisor).
- (+) No new crates — plain `std::process`.
- (−) Relies on the terminal delivering `SIGINT` to the process group for the clean Ctrl-C path;
  a hard, non-terminal kill of `gize` itself is not intercepted (documented — Ctrl-C is the
  supported stop).
- (−) Node/npm is now a soft runtime requirement for the admin path (documented; the API path is
  unaffected).

## Consequences

- `gize serve` takes `--api-only` / `--admin-only` / `--with-admin`; the run decision is a pure,
  unit-tested function of the flags and admin availability.
- Package-manager detection prefers pnpm, then npm, then yarn.
- For production, the admin is still built and deployed as a separate static artifact (ADR-006);
  `gize serve` is a development convenience.
- Windows is untested for the dual-process path; macOS/Linux are supported.

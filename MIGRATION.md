# Migration & upgrade guide

How to upgrade the `gize` CLI and bring an existing project up to a newer version **without
losing hand edits**. Read alongside [`STABILITY.md`](./STABILITY.md) (what is and isn't stable)
and [`CHANGELOG.md`](./CHANGELOG.md) (per-version notes).

## The core guarantee

You own the generated code. `gize sync` reconciles a project against its `gize.toml`, but it is
**drift-aware**: a file that exists on disk and differs from what the current templates would
generate is reported as **drift** and **left untouched** — it is never overwritten unless you
pass `--force` ([ADR-009](./ADR/adr-009-configuration.md), [ADR-012](./ADR/adr-012-cli.md)).

This holds across a **version upgrade** too. After you upgrade the CLI, a file can differ from
the new templates for two reasons:

1. you hand-edited it, or
2. the generator's output changed between versions — the exact bytes of generated code are
   explicitly **not** stable ([`STABILITY.md`](./STABILITY.md)).

`gize sync` treats both the same way: **report, don't clobber.** So upgrading `gize` and running
`sync` never silently rewrites code on disk. This is enforced by a regression test
(`sync_after_a_version_upgrade_preserves_on_disk_code` in
`crates/gize-generator/tests/generation.rs`).

## Upgrade flow

1. **Commit first.** Start from a clean git tree so any change `sync` proposes is reviewable as a
   diff (and revertible).

2. **Upgrade the CLI.**

   ```sh
   cargo install gize --force          # latest
   cargo install gize --version X.Y.Z  # or pin a version
   gize --version                      # confirm
   ```

3. **Preview what the new version wants.** `--dry-run` writes nothing:

   ```sh
   gize sync --dry-run
   ```

   - `create` — files the manifest declares that are missing on disk (safe to add).
   - `drift`  — files that differ from the new templates: **your edits and/or new-version
     output**. Nothing here is written without `--force`.
   - `ok`     — files that already match.

4. **Reconcile the drift.** Decide per file:

   - **Files you have *not* hand-edited** (drift is purely the new-version output): adopt the new
     template with `gize sync --force`. Review the resulting diff before committing.
   - **Files you *have* hand-edited**: `--force` would overwrite them, so reconcile by hand —
     re-apply your changes on top of the regenerated version, or keep yours. A practical pattern
     is to `--force` on a scratch branch, then cherry-pick the template changes into your edited
     files.

   > `--force` is a blunt instrument: it overwrites **every** drifted file. Run `--dry-run`
   > first and know which files carry your edits.

5. **Reconcile the database schema.** Model/manifest changes become migrations
   ([ADR-011](./ADR/adr-011-migrations.md)):

   ```sh
   gize make migration    # diff models → ALTER TABLE for new columns (drops need --force)
   gize migrate           # apply; gize migrate --status to inspect
   ```

6. **Re-run the gates.** A generated project stays compile-, clippy- and rustfmt-clean; verify
   after reconciling:

   ```sh
   gize check   # cargo clippy --all-targets -D warnings
   gize fmt
   cargo test
   ```

## Per-version notes

Most upgrades need only the flow above. Versions that change the **generated-code contract**
(not just bytes) are called out here and in the changelog; regenerating those files (`--force`,
then reconcile) adopts the new behavior.

### → 0.8.3 — `users` resource is admin-gated by default ([ADR-021](./ADR/adr-021-authorization.md))

The generated `users` resource changed from "reads public, writes require any valid token" to
**admin-gated end to end**: every `users` route except `register`/`login` — *including reads* —
now requires an admin token (`require_admin`, 401 without a token / 403 without the admin flag).
Generic `make crud` resources are unchanged (writes require auth, reads public).

If you regenerate the `users` slice (`gize sync --force` on `src/app/users/*` and
`src/auth/mod.rs`):

- Clients that previously read `GET /users` unauthenticated now get **401**; non-admin callers
  get **403**. Create an admin with `gize createadmin` and send an admin token.
- The JWT now carries an `is_admin` claim (set at login); a `require_admin` guard is generated
  alongside `require_auth`. Old tokens issued before the upgrade lack the claim — users must log
  in again.
- Keeping the old behavior is a supported choice: don't regenerate `src/app/users/routes.rs`, or
  edit it back to `require_auth`/public reads. `sync` will report it as drift and leave it alone.

See [`SECURITY.md`](./SECURITY.md) for the full authorization model.

## If an upgrade breaks something

If `gize sync` proposes to overwrite a file you did **not** ask it to, or a compatible upgrade
breaks a project in a way not described here or in the changelog, that is a bug — please open an
issue with both versions and a minimal reproduction.

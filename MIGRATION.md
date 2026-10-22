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
   cargo install gize --locked --force          # latest
   cargo install gize --locked --version X.Y.Z  # or pin a version
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

### → 0.9.0 — names are validated before generation

Generation (`make model`, `make crud`, `make app`, `make admin`, `sync`) now validates model,
field, relationship and module names first, and writes nothing if one is invalid, even with
`--force` or `--dry-run`. Rejected names include Rust keywords, SQL words reserved by the
selected database (`array` on PostgreSQL, `key` or `range` on MySQL), declared `id`,
`created_at` or `updated_at`, and duplicate or colliding fields. The error names the module to
rename.

A `gize.toml` with such names still works with `migrate`, `serve` and `check`; only generation
refuses it. To keep generating, rename the module or field in `gize.toml` and in your code, and
add a named migration (`gize make migration rename_...`) for the database side. Do not edit
migrations that were already applied. See the naming rules in
[the cookbook](./docs/cookbook.md#generate-a-resource).

### → 0.9.1 — paginated list routes ([ADR-022](./ADR/adr-022-pagination.md))

`GET /<table>` (and `GET /users`) now returns a page object instead of every row as an array:

```json
{ "items": [ ... ], "page": 1, "per_page": 25, "total": 130 }
```

It accepts `?page=` (1-based) and `?per_page=` (1 to 100, default 25); anything else is a
**422**. Rows come back ordered by `created_at DESC, id DESC`.

Nothing changes in an existing project until you regenerate it. To adopt pagination in a
module, regenerate its `dto.rs`, `repository.rs`, `service.rs` and `handler.rs` (for `users`,
the same four files under `src/app/users/`) with `gize sync --force`, then reconcile any hand
edits from your git diff. Afterwards:

- Update API clients to read `items` from the response, and to request further pages when
  `total` is larger than `per_page`.
- If the project has an admin, regenerate `admin/src/api.ts` in the same step: the new admin
  expects the page object and the old one expects an array. `gize sync --force` already
  rewrites it; `gize make admin --force` does the same on its own.
- `openapi.json` is refreshed by `sync` from the manifest, so it documents the new parameters.

Keeping the unpaginated routes is a supported choice: leave those files as they are and `sync`
will report them as drift without touching them.

### → 0.9.2 — password hashing off the async runtime

No API change. To adopt it, regenerate `src/auth/mod.rs` **and** `src/app/users/handler.rs`
together (`gize sync --force`, then reconcile hand edits from your git diff): the new handler
calls `hash_password_async` / `verify_password_async`, which only exist in the new auth module.
If you wrote handlers of your own that hash or verify passwords, switch them to the `_async`
helpers too. To change how many hashes run at once, edit `hashing_permits()` in
`src/auth/mod.rs`.

### → 0.9.3 — Rust 1.85 builds and MySQL foreign-key errors

No API change. Two fixes you can adopt by hand without regenerating:

- If a project must build on Rust 1.85 to 1.87, replace `jsonwebtoken = "9"` in its
  `Cargo.toml` with
  `jsonwebtoken = { version = "10", default-features = false, features = ["rust_crypto"] }`.
  The generated auth code compiles unchanged against it, and existing tokens stay valid.
- On MySQL, add `1451 | 1216 | 1217` to the `1452` arm in each module's `error.rs` so deleting
  a still-referenced row returns 409 instead of 500. `gize sync --force` regenerates these
  files with the change.

### → 0.9.4 — install with `--locked`; MySQL timestamps; `belongs_to` order

- Install and upgrade with `cargo install gize --locked`. Without `--locked` the install can
  fail on Rust older than the newest dependencies require.
- `gize make crud` / `make model` now refuse a `belongs_to` whose target is not in `gize.toml`.
  Generate the referenced resource first. Projects that already have such a migration order
  are not changed by this; if `gize migrate` fails with a missing referenced table, rename the
  dependent migration file so its timestamp comes after the referenced table's migration
  (only for migrations that were never applied).
- **MySQL only.** New tables store `created_at` / `updated_at` with microseconds. To upgrade
  an existing table, add a named migration per table and regenerate `repository.rs` so updates
  use `NOW(6)`:

  ```sql
  ALTER TABLE posts
    MODIFY created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    MODIFY updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6);
  ```

  Existing rows keep their whole-second values. Without this, everything still works; only
  the order of rows created within the same second is arbitrary.
- If the project has an admin, `gize sync --force` (or `gize make admin --force`) picks up the
  label fixes.

### → 0.9.6 — the model name is recorded in `gize.toml`

`gize make crud` now writes `model = "..."` to a module when its struct name cannot be derived
from the module name, so `gize sync` regenerates `Movie` instead of `Movy`. The CLI fills it in
when it creates or updates a module; a project created before 0.9.6 with such a model (the
module name ends in `ies` but the model ends in `ie`: `Movie`, `Cookie`, `Zombie`) needs it
added once by hand before running `sync`:

```toml
[[module]]
name = "movies"
model = "Movie"
```

Nothing else changes: other manifests stay valid as they are.

### → 0.9.11 — `register` without `is_admin`

Optional. New projects accept `POST /users/register` with just `name`, `email` and
`password`. An existing project keeps requiring `is_admin` in that body (and ignoring its
value) until `src/app/users/dto.rs` and `src/app/users/handler.rs` are regenerated: replace them
with the ones a 0.9.11 project generates, or, if `gize sync --dry-run` lists only those files as
drift, run `gize sync --force`. Clients that send `is_admin` work with both versions.

## If an upgrade breaks something

If `gize sync` proposes to overwrite a file you did **not** ask it to, or a compatible upgrade
breaks a project in a way not described here or in the changelog, that is a bug — please open an
issue with both versions and a minimal reproduction.

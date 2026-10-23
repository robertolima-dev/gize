# Cookbook

Task-sized recipes. Each is self-contained and assumes you are inside a Gize project. For a
guided first run, start with [Getting started](./getting-started.md); for the full command
reference see the [README](../README.md).

## Generate a resource

```sh
gize make crud Product name:String price:i32 active:bool
```

Produces the full layered slice (model, dto, repository, service, handler, routes, error, tests),
wires it into `src/app/mod.rs`, records it in `gize.toml`, and writes a `CREATE TABLE` migration.
Field types: `String` (`str`), `bool`, `i32` (`int`), `i64` (`bigint`/`long`), `f64`
(`float`/`double`), `Uuid`, `DateTime` (`timestamp`); every model also gets `id`, `created_at`,
`updated_at`.

Names are validated before generation. Models use ASCII PascalCase (`BlogPost`);
fields, relationship names/targets and manifest modules use lowercase ASCII identifiers
(`first_name`, `author`, `users`). Digits are allowed after the first character and an
initial underscore is allowed, but `_` alone, paths, spaces and raw Rust identifiers are
rejected. SQL identifiers are limited to 63 bytes to avoid PostgreSQL truncation.
Rust keywords and SQL names reserved for the selected database are rejected where they
would be emitted. For example, `key` is allowed on PostgreSQL and SQLite, but not MySQL;
`array` and `only` are rejected on PostgreSQL, `range` and `rows` on MySQL. The lists cover
every keyword that breaks Gize's unquoted SQL on PostgreSQL 14+, MySQL 5.7/8.x and SQLite;
`python3 scripts/probe_reserved_words.py` re-checks them against local servers.
Code-only `make app user` does not require a SQL table identifier.

Do not declare `id`, `created_at` or `updated_at`: Gize provides them. Field names must be
unique, and `author:belongs_to:users` already supplies `author_id:Uuid`, so declaring that
column separately is an error. Duplicate relationships and collisions between relationship
names and fields are also rejected. Some model names (`String`, `Result`, `Error`, etc.)
conflict with types used in generated code and are rejected with an explanation.

The same checks apply to `gize.toml`, including duplicate module names. All modules are
validated before writing files, even with `--force` or `--dry-run`. Existing manifests with
invalid names remain readable by `migrate`, `serve` and `check`; strict validation applies
to generation, not execution. Named manual migrations remain available as an escape hatch.
Review any corresponding code/schema rename manually and do not rewrite applied migrations.

Offline schema replay accepts enum declarations and drops of functions/triggers without
`CASCADE`. `DROP EXTENSION` cannot be parsed by sqlparser 0.53 and still blocks automatic
diffing; manually authored migrations can still be applied with `gize migrate`. Intentional
type differences continue to warn on each diff; there is currently no acknowledgment setting.

## Add a field to an existing resource

1. Add it to the module's `fields` in `gize.toml` (or re-run `make crud … --force` after editing).
2. Emit and apply the schema change:

   ```sh
   gize make migration     # diffs models → ALTER TABLE ADD COLUMN (new columns are nullable + TODO)
   gize migrate
   ```

3. Reconcile the code (never clobbers your edits without `--force`):

   ```sh
   gize sync --dry-run
   gize sync --force       # only for files you have not hand-edited
   ```

Column **drops** are withheld unless you pass `--force` to `make migration`.

After adding a column, review its generated backfill TODO before using the required Rust
field against existing rows. Until the column is made `NOT NULL`, subsequent diffs warn
about the mismatch without generating another ADD. Use `ALTER COLUMN ... SET NOT NULL`
on PostgreSQL, `MODIFY COLUMN ... <type> NOT NULL` on MySQL, or an explicit table rebuild
on SQLite after filling existing NULL values.

For a **type difference**, `gize make migration` warns about the existing SQL type and the
generator default, preserving the existing type even with `--force`. All modules are checked,
and independent additions still proceed. Custom choices such as `VARCHAR(120)` can remain;
verify Rust/SQL compatibility, especially timestamp timezones and numeric precision. If you
intend a conversion, create a named migration:

```sh
gize make migration change_product_price_type
# Edit the migration: review conversion, backfill and constraints for your database.
gize make migration  # replays the manual change; warns if it still differs from the default
gize migrate
```

Diffing replays forward migration files in numeric version order, ignoring `.down.sql`.
Drops and explicit renames are remembered; removed columns can be reintroduced without
repeating an old DROP. Unsupported SQL stops automatic diffing with the filename rather
than assuming synchronization. Named migrations and `gize migrate` still work; see
[the supported replay scope](../ADR/adr-011-migrations.md#revision-2026-10-20--replay-forward-schema-history).

## Add a relationship (belongs-to / 1-N)

```sh
gize make crud Comment body:String post:belongs_to:posts author:belongs_to:users
```

`field:belongs_to:target` adds the foreign-key column and `FOREIGN KEY` constraint, and orders
migrations topologically so each target table is created before the table that references it. A
cycle is a hard error.

## Protect a route / require an admin

Generated resources already pick a guard in `src/app/<resource>/routes.rs`. To change it, apply a
guard from `src/auth` as a route layer:

```rust
use crate::auth::{require_auth, require_admin};
use axum::middleware;

// any authenticated user:
let protected = Router::new()
    .route("/reports", get(handler::list))
    .route_layer(middleware::from_fn(require_auth));

// admins only (403 for non-admins):
let admin = Router::new()
    .route("/reports/:id", delete(handler::delete))
    .route_layer(middleware::from_fn(require_admin));
```

Defaults: the `users` resource is admin-gated end to end; generic resources guard writes with
`require_auth` and keep reads public. See [SECURITY.md](../SECURITY.md).

## Ownership ("users edit only their own records")

Not auto-generated — it is domain-specific. Compare the token subject to the record in your
handler/service. The JWT `sub` claim is the user id:

```rust
use crate::auth::verify_token; // or extract Claims via your guard

// pseudo: reject if the authenticated user id != the resource owner
if claims.sub != post.author_id.to_string() {
    return Err(Error::Unauthorized);
}
```

## Create an admin user

```sh
# interactive
gize createadmin --email admin@example.com --name Admin

# non-interactive / CI (password from an env var, never a CLI arg)
ADMIN_PW='...' gize createadmin --email admin@example.com --name Admin --password-env ADMIN_PW
```

## Choose a database (Postgres / SQLite / MySQL)

Pick at scaffold time:

```sh
gize new shop                      # Postgres (default)
gize new shop --database sqlite    # no server; great for tutorials/tests
gize new shop --database mysql
```

The dialect drives SQL types, placeholders, UUID handling and `RETURNING`
([ADR-015](../ADR/adr-015-second-database.md)). Set the matching `DATABASE_URL` in `.env`:

```sh
DATABASE_URL=postgres://user@localhost/shop
DATABASE_URL=sqlite://shop.db?mode=rwc
DATABASE_URL=mysql://user:pass@localhost/shop
```

## Enable OpenAPI docs

```sh
gize new shop --openapi        # at scaffold time
```

Serves `GET /openapi.json` and a docs UI at `/docs`, generated from the manifest so it matches
your routes by construction ([ADR-010](../ADR/adr-010-openapi.md)). For an existing project, set
`openapi = true` under `[features]` in `gize.toml` and run `gize sync`.

## Enable the admin UI

```sh
gize make admin        # generates a Vite + React SPA (admin/) for your resources
gize serve             # serves the API and, when an admin exists, the admin dev server
```

`gize serve --api-only` runs just the API; `--admin-only` just the admin. See
[ADR-006](../ADR/adr-006-admin.md).

## Add a WebSocket endpoint

```sh
gize new chat --ws      # scaffolds src/app/ws/ with a typed echo endpoint (ADR-018)
```

For an existing project, set `websocket = true` under `[features]` and `gize sync`.

## Version your API

```sh
gize new api --api-version 1   # routes served under /api/v1/...
```

Records `[api] version = "v1"` in the manifest; the OpenAPI paths carry the same prefix
([ADR-016](../ADR/adr-016-api-versioning.md)).

## Customize validation

Edit `src/app/<resource>/dto.rs` — the DTOs use [`validator`](https://docs.rs/validator):

```rust
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProduct {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub name: String,
    #[validate(range(min = 0, message = "must be non-negative"))]
    pub price: i32,
}
```

Validation failures return **422** with field messages; a unique-constraint violation returns
**409**, a missing/again-referenced FK also **409**.

## Page through a list

Every generated list route is paginated ([ADR-022](../ADR/adr-022-pagination.md)):

```sh
curl 'localhost:8080/products?page=2&per_page=50'
# → {"items":[...],"page":2,"per_page":50,"total":137}
```

`page` starts at 1 and defaults to 1; `per_page` defaults to 25 and accepts up to 100. Out-of-range
or non-numeric values return **422**. Rows are ordered newest first (`created_at DESC, id DESC`),
so the order stays stable from page to page. A page past the end returns an empty `items` with
the real `total`.

To change the limits for one resource, edit `DEFAULT_PER_PAGE` / `MAX_PER_PAGE` in its `dto.rs`.
To sort or filter differently, edit the query in its `repository.rs`; the code is yours.

## Rebuild a project from the manifest

Deleted a module's code, or cloned a repo that only committed `gize.toml`?

```sh
gize sync            # recreates missing files and migrations from the manifest
gize sync --force    # also overwrites drifted files (review the diff first!)
```

## Diagnose problems

```sh
gize doctor          # checks .env, DATABASE_URL, GIZE_JWT_SECRET, project layout
gize check           # cargo clippy --all-targets -D warnings
gize fmt             # rustfmt
```

## Upgrade Gize safely

See **[MIGRATION.md](../MIGRATION.md)** — upgrade the CLI, `gize sync --dry-run` to preview, then
reconcile drift (`--force` for untouched files, merge by hand for edited ones).

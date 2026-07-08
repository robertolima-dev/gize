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
gize new chat --websocket      # scaffolds src/app/ws/ with a typed echo endpoint (ADR-018)
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

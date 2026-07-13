# Getting started

A linear, ten-minute tutorial: from an empty directory to a running, authenticated CRUD API
with a relationship. It uses **SQLite** so you need no database server; everything transfers to
Postgres or MySQL by changing one flag.

For the exhaustive command/flag reference see the [README](../README.md); for task-sized recipes
see the [Cookbook](./cookbook.md); for how it all fits together see the
[Architecture](./architecture.md).

## Prerequisites

- **Rust 1.85+** (edition 2024).
- The Gize CLI:

  ```sh
  cargo install gize
  gize --version
  ```

## 1. Scaffold a project

```sh
gize new blog --database sqlite
cd blog
```

`gize new` creates an idiomatic Axum + SQLx project and, by default, a complete **`users`**
resource (model, CRUD, migration, and Argon2 + JWT auth). Opt out with `--no-user`.

Look around — this is plain Rust you own:

```
src/
  main.rs router.rs state.rs
  config/   auth/            # JWT + password hashing (you own it)
  app/
    users/  mod.rs model.rs dto.rs repository.rs service.rs handler.rs routes.rs error.rs tests.rs
migrations/
gize.toml                    # the manifest: source of truth for `gize sync`
```

## 2. Add a resource with a relationship

Generate a `Post` that belongs to a user. The `field:belongs_to:target` syntax adds the foreign
key and orders the migrations so the target table exists first.

```sh
gize make crud Post title:String body:String published:bool author:belongs_to:users
```

Every model automatically gets `id: Uuid` plus `created_at` / `updated_at`. This wires the module
into `src/app/mod.rs`, records it in `gize.toml`, and writes a `CREATE TABLE` migration.

## 3. Configure and migrate

```sh
cp .env.example .env
```

Edit `.env` — for SQLite:

```sh
DATABASE_URL=sqlite://blog.db?mode=rwc
PORT=8080
GIZE_JWT_SECRET=change-me-to-a-long-random-string
```

Apply the migrations (inspect first with `--status`):

```sh
gize migrate --status
gize migrate
```

## 4. Create an admin

The `users` resource is **admin-gated by default** ([ADR-021](../ADR/adr-021-authorization.md)):
managing users and listing them requires an admin token. Create the first admin:

```sh
gize createadmin --email admin@example.com --name Admin
```

## 5. Run it

```sh
gize serve
```

The API is on `http://localhost:8080`. In another terminal:

```sh
# public: register a normal user and get a token
curl -s -X POST localhost:8080/users/register -H 'content-type: application/json' \
  -d '{"name":"Ada","email":"ada@example.com","password":"longenough","is_admin":false}'

# public: log in an existing account (e.g. the admin from `gize createadmin`) — same token shape
curl -s -X POST localhost:8080/users/login -H 'content-type: application/json' \
  -d '{"email":"ada@example.com","password":"longenough"}'

# public: reads on a generic resource
curl -s localhost:8080/posts

# writes require a token — grab one from register/login, then:
TOKEN=... # the "token" field from the response above
curl -s -X POST localhost:8080/posts -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"title":"Hello","body":"First post","published":true,"author_id":"<a user id>"}'
```

What you will see (the generated authorization model):

- `POST /posts` without a token → **401**; with any valid token → **201**.
- `GET /users` → **401** without a token, **403** for a non-admin, **200** only for an admin.
- `GET /users/me` → **401** without a token; with any valid token → **200** with the caller's
  own record (self-service, identified by the token's `sub` claim — no admin flag required).
- `GET /posts` (a generic resource) → **public**.
- A registered user is never an admin; passwords are hashed and never serialized.

## 6. Change the shape, re-sync

Add a field by editing the module in `gize.toml` (or re-running `make crud`), then reconcile:

```sh
gize make migration     # diff models → ALTER TABLE for the new column
gize migrate
gize sync --dry-run     # preview any code the manifest now implies (never clobbers your edits)
```

`gize sync` is **drift-aware**: it never overwrites a file you changed without `--force`.

## Keep the code clean

```sh
gize check   # cargo clippy --all-targets -D warnings
gize fmt     # rustfmt
cargo test
```

## Where to next

- **[Cookbook](./cookbook.md)** — protect routes, add OpenAPI/admin/WebSocket, switch databases,
  version your API, custom validation.
- **[Architecture](./architecture.md)** — how generation, the manifest, and `sync` work.
- **[FAQ](./faq.md)** — is it an ORM? does it lock me in? is the auth production-ready?
- **[MIGRATION.md](../MIGRATION.md)** — upgrading Gize without losing hand edits.
- Going to production? Read **[SECURITY.md](../SECURITY.md)** (authorization, CORS, secrets).

# RC validation walkthrough

A scripted, ~15-minute walkthrough for **independently validating a Gize release candidate**:
install from crates.io as a new user would, build a non-trivial app that touches the whole
surface, and confirm it behaves as documented. It closes the RC acceptance criterion *"a team
outside the core builds an app on the RC"* ([roadmap](./roadmap.md)).

When you're done, please file a report using the **RC validation report** issue template.

## 0. Prerequisites

- **Rust 1.85+** (`rustc --version`).
- Install the CLI **from crates.io** (a cold install — this is part of what we're validating):

  ```sh
  cargo install gize
  gize --version
  ```

This walkthrough uses **SQLite** so you need no database server. To validate a more realistic
setup, repeat it with `--database postgres` (or `mysql`) and a running server.

## 1. Scaffold with a feature enabled

```sh
gize new shop --database sqlite --openapi
cd shop
```

**Expect:** a project tree under `src/` with a built-in `users` resource and an `auth/` module;
`gize.toml` recording the project; `openapi = true` under `[features]`.

## 2. Add a resource with a relationship

```sh
gize make crud Product name:String price:i32 active:bool
gize make crud Review body:String rating:i32 product:belongs_to:products author:belongs_to:users
```

**Expect:** each command creates a full slice under `src/app/…`, wires it into `src/app/mod.rs`,
records it in `gize.toml`, and adds a `CREATE TABLE` migration. The `Review` migration is ordered
*after* `products` and `users` (its foreign keys).

## 3. Configure, migrate, seed an admin

```sh
cp .env.example .env
```

Edit `.env`:

```sh
DATABASE_URL=sqlite://shop.db?mode=rwc
PORT=8080
GIZE_JWT_SECRET=a-long-random-string-for-validation
```

```sh
gize migrate --status     # expect: pending migrations listed
gize migrate              # expect: applied
gize createadmin --email admin@example.com --name Admin   # set a password when prompted
```

## 4. Run it and exercise the routes

```sh
gize serve                # API on http://localhost:8080 (+ admin dev server if you ran make admin)
```

In another terminal — confirm the documented authorization model:

```sh
code() { curl -s -o /dev/null -w '%{http_code}\n' "$@"; }

# public: reads on a generic resource
code localhost:8080/products                         # expect 200

# generic-resource writes require a token
code -X POST localhost:8080/products -d '{}'         # expect 401

# register a normal user (public) and grab the token from the JSON body
curl -s -X POST localhost:8080/users/register -H 'content-type: application/json' \
  -d '{"name":"Ada","email":"ada@example.com","password":"longenough","is_admin":false}'

# users resource is admin-gated (ADR-021)
code localhost:8080/users                            # expect 401 (no token)
code localhost:8080/users -H "authorization: Bearer <ada-token>"    # expect 403 (not admin)
# log in as the admin, then:
code localhost:8080/users -H "authorization: Bearer <admin-token>"  # expect 200
```

**Also check:**

- A registered user is stored as **non-admin** even if the body says `is_admin: true`.
- Responses never contain a `password` field.
- OpenAPI: open `http://localhost:8080/docs` and fetch `http://localhost:8080/openapi.json`.

## 5. (Optional) Admin UI, other databases, versioning

```sh
gize make admin           # generates a Vite + React admin SPA; `gize serve` serves it too
```

Repeat steps 1–4 with `--database postgres`/`mysql`, or scaffold with `--api-version 1` (routes
under `/api/v1/...`) or `--websocket`.

## 6. Regeneration safety

```sh
# make a hand edit
echo '// my custom logic' >> src/app/products/service.rs
gize sync --dry-run       # expect: products/service.rs reported as "drift", nothing written
```

**Expect:** your edit is reported as drift and **not** overwritten (it would only change with
`--force`). See [MIGRATION.md](../MIGRATION.md).

## 7. Quality gates

```sh
gize check                # cargo clippy --all-targets -D warnings — expect clean
gize fmt                  # rustfmt — expect no changes
cargo test                # the generated tests compile and pass
```

## Acceptance checklist

- [ ] Cold `cargo install gize` worked.
- [ ] Scaffolding + `make crud` (incl. a `belongs_to`) produced a compiling project.
- [ ] `migrate`, `createadmin`, `serve` worked end to end.
- [ ] Auth matrix matched: 401 without a token, 403 for a non-admin on `users`, 200 for an admin,
      201 on an authenticated generic write; passwords never serialized.
- [ ] OpenAPI (if enabled) served and matched the routes.
- [ ] `gize sync` preserved a hand edit.
- [ ] Generated project is clippy- and rustfmt-clean and its tests pass.
- [ ] The docs ([Getting started](./getting-started.md) / [Cookbook](./cookbook.md) /
      [FAQ](./faq.md)) were enough to get unstuck.

## Report

Open an issue with the **RC validation report** template and paste your results (including any
friction or papercuts — small things count). Bugs go under the **Bug report** template.

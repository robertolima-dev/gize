# Security

## Reporting a vulnerability

If you find a security issue in Gize or in the code it generates, please report it privately:
open a [GitHub security advisory](https://github.com/robertolima-dev/gize/security/advisories/new)
(or email the maintainer) with the version, a description, and a minimal reproduction. Please do
not open a public issue for undisclosed vulnerabilities. We aim to acknowledge reports within a
few days.

## The security model of generated apps

Gize generates plain, owned Rust — there is no hidden runtime. The generated auth stack
([ADR-013](./ADR/adr-013-auth.md)) gives you a solid baseline, but some decisions are inherently
the application's responsibility. Read this before shipping.

### Authentication (what Gize gives you)

- **Passwords** are hashed with **Argon2id** (random per-password salt) and never stored or
  serialized in plaintext; the `password` field is `#[serde(skip_serializing)]`.
- **Sessions** are stateless **JWT (HS256)** signed with `GIZE_JWT_SECRET`, read from the
  environment only — never from `gize.toml`. There is **no insecure default**: if the secret is
  unset the app returns a 500 rather than signing with a weak key. Tokens carry a 24h expiry that
  is validated on every request, and the verifier pins HS256 (no algorithm-confusion).
- **Login is constant-time with respect to account existence** (since 0.8.1): an unknown email is
  checked against a throwaway hash, so response timing does not reveal which emails are
  registered.
- **`gize createadmin`** never takes the password as an argument — only a hidden prompt or an
  environment variable.

**Use a long, random `GIZE_JWT_SECRET` in production.** HS256 is only as strong as its secret.

### Authorization (your responsibility)

The generated `require_auth` middleware checks that a request carries a **valid token** — it does
**not** check roles or ownership. Out of the box, **any authenticated user can call any guarded
write route**, including modifying or deleting other users. The `users` table ships an `is_admin`
flag, but the generated routes do not gate on it.

Before production you must add the authorization your domain needs, for example:

- ownership checks (a user may edit only their own records), and/or
- role checks (gate admin routes on `is_admin`).

`register` already forces `is_admin = false` so a client cannot grant itself admin through the
public endpoint; admins are created via the guarded `POST /users` or `gize createadmin`.

### Public read routes

Generated `GET /<resource>` and `GET /<resource>/:id` are **public** by default (only writes are
guarded). For the `users` resource this exposes names, emails and the `is_admin` flag (never the
password hash) and allows listing all accounts. If that is not acceptable, move those routes into
the guarded router in `src/app/users/routes.rs`.

### Error responses

Database errors are **logged server-side and never returned to the client** (since 0.8.1) — the
client gets a generic `500` so schema, SQL and connection details do not leak. Validation errors
(422) do return field-level messages, which describe the client's own input.

### Deployment notes

- **CORS:** in development the admin talks to the API through a Vite dev proxy (same origin), so
  no CORS layer is generated. In production the admin is a separate static artifact on a
  different origin — add a CORS layer (e.g. `tower-http`'s `CorsLayer`) scoped to your admin's
  origin; do not allow any origin with credentials.
- **Admin token storage:** the generated React admin stores the JWT in `localStorage`, which is
  readable by any script on the page (an XSS risk). Keep the admin's dependencies and content
  trusted; a cookie-based session is a hardening option you own.
- **HTTPS/TLS** termination is expected to be handled by your platform or a reverse proxy.

## Hardening done in the RC review (0.8.1)

- Database errors are no longer returned to clients (logged server-side instead).
- Login no longer leaks account existence through response timing.
- Documented the authorization model, public-read exposure, CORS and token-storage tradeoffs
  above so they are explicit decisions, not surprises.

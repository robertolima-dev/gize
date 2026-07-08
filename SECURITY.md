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

### Authorization (ADR-021)

Gize generates two guards in `src/auth/mod.rs`:

- `require_auth` — the request carries a **valid token** (rejects with `401` otherwise).
- `require_admin` — the token additionally has `is_admin == true` (rejects with `401` when the
  token is missing/invalid, `403` when the caller is authenticated but not an admin). The admin
  flag is embedded in the token at login, so no per-request database read is needed.

**The `users` resource is admin-gated by default.** Every `users` route except `register`/`login`
— *including reads* — requires an admin token, so no authenticated user can list, edit or delete
other accounts, and account listings are not exposed. `register` still forces `is_admin = false`
so a client cannot grant itself admin; admins are created via the admin-guarded `POST /users` or
`gize createadmin`.

**Generic resources** (from `gize make crud`) keep the pragmatic default: **writes require
`require_auth`, reads are public** (see below). Role and ownership authorization there is still
**your responsibility** — the building blocks are provided:

- ownership checks (a user may act only on their own records) — compare the token's `sub` claim to
  the record; **not** auto-generated, since it is domain-specific;
- role checks — apply `require_admin` (or your own guard) to the routes that need it.

Because the admin flag lives in the token, changing a user's `is_admin` only takes effect on their
next login (tokens are short-lived; see `TOKEN_TTL_SECS`). For immediate revocation you need
server-side sessions (a tracked post-1.0 option).

### Public read routes

For **generic** resources, `GET /<resource>` and `GET /<resource>/:id` are **public** by default
(only writes are guarded) — the intended model for public data such as blog posts. If a given
resource must not be world-readable, move its read routes into the guarded router (wrap them with
`require_auth` or `require_admin`) in `src/app/<resource>/routes.rs`.

The `users` resource is the exception: its reads are **admin-gated** (ADR-021), so names, emails
and the `is_admin` flag are never exposed publicly and accounts cannot be enumerated.

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

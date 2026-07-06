# ADR-013: Authentication (gize-auth MVP)

- **Status:** Accepted
- **Date:** 2026-07-06
- **Deciders:** Gize core team

## Context

The MVP ships a `users` resource but stores passwords in plain text and protects nothing —
the backlog flags this explicitly as a follow-up. The Alpha needs a real, small auth story:
hash passwords, issue a credential on login, and let generated routes require an authenticated
user. This must be idiomatic Axum/SQLx code the developer owns (no hidden framework), and it
must pass a security review before shipping (roadmap risk).

`gize-auth` is a workspace crate that today is a placeholder.

## Alternatives

**Password hashing**

1. **argon2** (Argon2id) — modern, memory-hard, the current OWASP-recommended default.
2. **bcrypt** — battle-tested but older; 72-byte input cap, weaker against GPU attacks.
3. **scrypt / pbkdf2** — acceptable but not preferred over Argon2id for new code.

**Session credential**

1. **JWT (HS256), stateless** — signed token carrying user id + expiry; no server-side
   session store. Simple to generate, no per-request DB hit.
2. **Opaque token + `sessions` table** — server-side, revocable, but adds a table, a
   migration, and a DB read on every guarded request.
3. **Configurable (`token_type = jwt | session`)** — most flexible, doubles the template and
   test surface.

## Decision

**Argon2id** for password hashing (`argon2` crate, default params), via a `hash_password` /
`verify_password` pair in `gize-auth`. Hashing happens in the service layer on `create`/
`update`; the `password` field stays `#[serde(skip_serializing)]` so it never leaves in a
response (already true for the generated `users` model).

**JWT (HS256), stateless**, for the session credential:

- `gize-auth` provides `issue_token(user_id, …) -> String` and a Claims type (`sub`, `exp`,
  `iat`), signed with a secret from the environment (`GIZE_JWT_SECRET`), **never** from
  `gize.toml` (ADR-009 keeps secrets out of the manifest).
- An Axum extractor `AuthUser` (and a middleware guard) validates the `Authorization: Bearer`
  token and rejects with `401` on missing/invalid/expired tokens.
- Generated apps get `register` (hash + insert) and `login` (verify + issue) handlers on the
  `users` module, and write routes can be wrapped with the guard.

Configurable token types (`session`, PASETO, opaque) are **out of scope** for the Alpha and
remain the separate P2 backlog item; the seam (`features.authentication` growing into a
sub-config with `token_type`) is noted but not built now.

## Trade-offs

- (+) Argon2id + JWT are the boring, correct, widely-audited defaults.
- (+) Stateless JWT needs no session table or per-request DB read; simplest thing that works.
- (+) Plain Axum extractor/middleware — the developer can read and replace it.
- (−) **Revocation is limited**: a stateless JWT is valid until it expires. Mitigated with a
  short default expiry (e.g. hours) and documented; server-side revocation lists are a later
  concern, unlocked by the `session` token type.
- (−) Secret management is on the developer (env var); `gize doctor` should check it's set.

## Consequences

- `gize-auth` gains real dependencies (`argon2`, `jsonwebtoken`) and a small, tested API.
- `gize-templates::user` generates `register`/`login` and hashes on write; the plain-text-
  password follow-up in the backlog is closed.
- `features.authentication` in the manifest, when true, wires the guard onto write routes.
- **A security review is required before this ships** (weak-secret handling, timing-safe
  verify, no token/hash leakage in logs or responses, correct `exp` validation).
- `GIZE_JWT_SECRET` is added to `.env.example` and checked by `gize doctor`.

## Security review (Alpha, 2026-07-06)

Performed on the generated auth code before shipping WS5.

**Fixed:**

- **Privilege escalation via `register`.** The public `register` handler accepted `is_admin`
  from the request body, letting any client self-grant admin. `register` now forces
  `is_admin = false`; admins are created only through the guarded `POST /users`.
- **Password never serialized.** The `User` model keeps `#[serde(skip_serializing)]` on
  `password`; verified end-to-end that `GET /users` responses contain no password/hash.
- **Passwords hashed on every write path.** `register`, `create` and `update` all run
  `hash_password` (Argon2id); no plaintext reaches the database.
- **Token validation.** `exp` is enforced by `jsonwebtoken::Validation::default()`; an invalid
  or expired token yields `401`, verified at runtime.

**Accepted for Alpha / deferred (documented, not blocking):**

- **Public reads.** `GET /users` (and list/show generally) are unauthenticated, consistent
  with the "reads public, writes guarded" model. This exposes user names/emails; tightening
  read authorization is a Beta concern (roles/policies).
- **Login user-enumeration timing.** A login for a non-existent email returns before any hash
  verification, a minor timing side-channel. A constant-time dummy verify is a follow-up.
- **No login rate limiting / password-strength policy.** Out of scope for the Alpha; brute
  force and weak-password mitigation are tracked for later (rate limiting; validation layer).
- **Default dev secret.** `.env.example` ships `GIZE_JWT_SECRET=dev-only-change-me` with a
  note to replace it; `gize doctor` reports whether it is set.

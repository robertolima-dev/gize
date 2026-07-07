# Gize Chat — reference app

A tiny, real-time group chat built with [Gize](https://github.com/robertolima-dev/gize) to
show the framework end-to-end: **WebSockets, a persisted resource, the built-in users/auth, an
admin UI and SQLite** — all working together, in plain Rust you own.

## What it demonstrates

| Gize feature | In this app |
| --- | --- |
| `gize new --ws` | The `src/app/ws/` module (upgrade handler + typed messages). |
| `gize make crud Message` | `Message { content, username }` — a migration, SQLx repository, REST endpoints, and an entry in the admin. |
| Built-in `users` + auth | Argon2 + JWT scaffolding (ready for auth-gating the socket). |
| `gize make admin` | A React admin (`admin/`) to browse users and messages. |
| SQLite (`--database sqlite`) | Zero-setup persistence — clone and run. |

## Generated vs. hand-written

That distinction is the whole point of Gize. Everything above was **generated**; turning the
scaffold's *echo* into a *real broadcast chat* is a small, readable hand edit you own:

- `src/state.rs` — added a `tokio::sync::broadcast` channel to `AppState`.
- `src/app/ws/message.rs` — a single `ChatMessage { username, content }` type.
- `src/app/ws/handler.rs` — on each connection, `select!` between the client and the broadcast:
  persist incoming messages through the **generated** `messages` resource, then fan them out to
  every other connection.
- `src/app/ws/routes.rs` + `client.html` — serve a minimal chat UI at `/`.

No framework runtime, no magic: just Axum, SQLx and Tokio wired the way you'd write them.

## Run it

```bash
cp .env.example .env          # DATABASE_URL=sqlite://chat.db?mode=rwc
gize migrate                  # create the users + messages tables
gize serve                    # API on :8080 (and the admin dev server on :5173)
```

Open <http://localhost:8080/> in two browser tabs and chat — messages are broadcast live and
persisted. Browse them (and users) in the admin at <http://localhost:5173/>.

Create an admin login for the panel:

```bash
gize createadmin --email admin@example.com --name Admin --password-env GIZE_ADMIN_PASSWORD
```

### The WebSocket protocol

Send and receive JSON frames on `ws://localhost:8080/ws`:

```json
{ "username": "ana", "content": "hello!" }
```

Every message you send is persisted (`GET /messages`) and echoed to all connected clients.

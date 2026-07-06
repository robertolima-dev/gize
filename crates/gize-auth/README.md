# gize-auth

**Authentication conventions for the [Gize](https://github.com/robertolima-dev/gize) framework.**

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/robertolima-dev/gize#license)

Authentication shipped in the Alpha, but it is **generated inline** into each project (a
`src/auth` module with Argon2 password hashing, stateless JWT, a route guard, and
register/login on the built-in `users` resource), not pulled in as a library. That keeps the
auth code idiomatic and fully owned by you, with no hidden framework. See
[ADR-013](https://github.com/robertolima-dev/gize/blob/main/ADR/adr-013-auth.md).

> **Status:** reserved. This crate is not published; the auth implementation lives in the
> templates (`gize-templates::auth` / `::user`). The name is held for a future extraction, for
> example a configurable token type (JWT, PASETO, opaque/session), should that prove useful.

## Part of the Gize workspace

See the [main project](https://github.com/robertolima-dev/gize) for the full crate list and
roadmap.

## License

Licensed under either of [Apache-2.0](https://github.com/robertolima-dev/gize/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/robertolima-dev/gize/blob/main/LICENSE-MIT) at your option.

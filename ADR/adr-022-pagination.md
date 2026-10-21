# ADR-022: Paginated list endpoints

- **Status:** Accepted
- **Date:** 2026-10-21
- **Deciders:** Gize core team

## Context

Every generated resource exposes `GET /<table>`, which returns **every row** as a JSON array
(`SELECT * FROM <table> ORDER BY created_at DESC` + `fetch_all`). The built-in `users` resource
does the same. This has three problems:

- **Unbounded responses.** A table with a few hundred thousand rows produces a response (and a
  server-side `Vec`) of the same size on every call.
- **Unstable order.** Rows sharing a `created_at` value can come back in any order, so any
  client-side paging can skip or repeat records.
- **The contract is about to freeze.** The response shape of the list route is part of the
  generated-code contract that 1.0 puts under SemVer (`STABILITY.md`). Changing an array into
  a page object after 1.0 would be a major-version change. RC is the last window to decide it.

The generated admin SPA downloads the full list and filters, searches and pages it in the
browser. Foreign-key selects and cells also read the full list of the related resource.

## Alternatives

1. **Page envelope with `page` / `per_page`.** `GET /posts?page=2&per_page=25` returns
   `{ "items": [...], "page": 2, "per_page": 25, "total": 137 }`. The object can gain fields
   later without breaking clients. It changes the response from an array to an object.
2. **Array + `limit` / `offset`, total in `X-Total-Count`.** Smaller change to the response
   shape, but clients that expected "all rows" silently receive the first page only, and the
   total lives outside the JSON body (invisible to many HTTP clients and to the OpenAPI body).
3. **Cursor / keyset pagination** (`?cursor=` → `{ items, next_cursor }`). Scales best and is
   stable under concurrent inserts, but offers no total and no jump to page *N*, and needs an
   opaque cursor encoding in every generated module. More code than a CRUD starter warrants.

## Decision

**Option 1: a page envelope with `page` / `per_page`.**

- Query parameters: `page` (1-based, default `1`) and `per_page` (default `25`, maximum `100`).
- Response body: `{ "items": [...], "page": <u32>, "per_page": <u32>, "total": <u64> }`, where
  `total` is the number of rows in the table.
- `page < 1`, `per_page < 1`, `per_page > 100`, or a non-integer value return **422** with a
  message, consistent with the other validation errors of the generated resources.
- Order is stable: `ORDER BY created_at DESC, id DESC`.
- SQL: `LIMIT` / `OFFSET` with bound parameters plus a `SELECT COUNT(*)`. Both are portable
  across PostgreSQL, SQLite and MySQL.
- The pagination types (`ListParams`, `Page<T>`) are generated **inside each module's
  `dto.rs`**. A shared module would break `gize make crud` in projects generated before it
  existed, and each module stays self-contained (ADR-005).
- The OpenAPI spec documents both query parameters, the `<Name>Page` response schema and the
  422 response (ADR-010).
- The admin SPA requests pages of 100 until it has every row, so search, filters and
  foreign-key selects keep working in the browser without new server-side query parameters.

Filtering, sorting and search parameters on the server are **not** part of this decision;
they would be new features under the 0.8.0 freeze.

## Consequences

- **Breaking for regenerated code and API clients:** `GET /<table>` returns an object instead
  of an array. Existing projects keep their current code (`gize sync` never overwrites without
  `--force`); new modules use the page envelope. `MIGRATION.md` documents the change.
- A list call costs two queries (page + count). For very large tables `COUNT(*)` is a full
  scan on some engines; developers who need otherwise own the generated repository.
- `OFFSET` gets slower on deep pages. That is acceptable for the generated default; keyset
  pagination remains possible by editing the owned code.
- The admin still loads every row, now in bounded requests. Server-side filtering for the
  admin can come later without changing this contract.

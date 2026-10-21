#!/usr/bin/env bash
# End-to-end test of a generated app against a real database: generate → migrate → serve →
# call every route, then evolve the schema (add a field, diff, backfill) and call the routes
# again. Runtime SQL errors surface as failed HTTP checks, not only as compile errors.
#
# Usage: GIZE=target/debug/gize DATABASE=sqlite|postgres|mysql [DATABASE_URL=...] scripts/e2e.sh
#
# DATABASE_URL is required for postgres and mysql and must point at an empty database; SQLite
# uses a file inside the generated project. The app listens on $PORT (default 8080).
set -euo pipefail

GIZE="$(cd "$(dirname "${GIZE:?set GIZE to the gize binary}")" && pwd)/$(basename "$GIZE")"
DATABASE="${DATABASE:?set DATABASE to sqlite, postgres or mysql}"
PORT="${PORT:-8080}"
BASE="localhost:$PORT"
case "$DATABASE" in
  sqlite) DATABASE_URL="sqlite://blog.db?mode=rwc" ;;
  postgres | mysql) : "${DATABASE_URL:?set DATABASE_URL for $DATABASE}" ;;
  *) echo "unknown DATABASE: $DATABASE" >&2; exit 2 ;;
esac

WORK="$(mktemp -d)"
SRV=""
stop_app() { if [ -n "$SRV" ]; then kill "$SRV" 2>/dev/null || true; wait "$SRV" 2>/dev/null || true; SRV=""; fi; }
trap stop_app EXIT

step() { echo "== $*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
# JSON bodies built with double quotes go through a variable first: `{...,...}` inside an
# unquoted-looking "$(...)" is brace-expanded by bash and silently shifts the arguments.
expect() { # expect <label> <actual> <expected>
  [ $# -eq 3 ] || fail "expect got $# arguments (a brace-expanded JSON body?): $*"
  [ "$2" = "$3" ] || fail "$1: got '$2', expected '$3'"
  echo "   ok  $1 = $2"
}
code() { curl -s -o /dev/null -w '%{http_code}' "$@"; }
json() { python3 -c "import sys,json; d=json.load(sys.stdin); print($1)"; }
migrations() { find migrations -name '*.sql' | wc -l | tr -d ' '; }
start_app() {
  cargo build -q
  "${CARGO_TARGET_DIR:-target}/debug/blog" >"$WORK/app.log" 2>&1 &
  SRV=$!
  for _ in $(seq 1 60); do curl -sf "$BASE/posts" >/dev/null 2>&1 && return; sleep 1; done
  cat "$WORK/app.log" >&2
  fail "the app did not start"
}

step "generate ($DATABASE)"
cd "$WORK"
"$GIZE" new blog --database "$DATABASE" >/dev/null
cd blog
"$GIZE" make crud Post title:String published:bool author:belongs_to:users >/dev/null
cat >.env <<EOF
DATABASE_URL=$DATABASE_URL
PORT=$PORT
GIZE_JWT_SECRET=e2e-secret-e2e-secret-e2e-secret
EOF
set -a; . ./.env; set +a

step "migrate (twice: the second run must be a no-op)"
"$GIZE" migrate >/dev/null
"$GIZE" migrate >/dev/null
ADMIN_PW='admin-password-123' "$GIZE" createadmin --email admin@e2e.dev --name Admin \
  --password-env ADMIN_PW >/dev/null

step "serve and call every route"
start_app
J='content-type: application/json'
token_of() { json "d['token']"; }
expect "write without token" "$(code -X POST "$BASE/posts" -H "$J" -d '{}')" 401
TOKEN=$(curl -s -X POST "$BASE/users/register" -H "$J" \
  -d '{"name":"Ada","email":"ada@e2e.dev","password":"longenough","is_admin":true}' | token_of)
expect "register forces is_admin=false" \
  "$(curl -s "$BASE/users/me" -H "authorization: Bearer $TOKEN" | json "d['is_admin']")" False
expect "users list without token" "$(code "$BASE/users")" 401
expect "users list as non-admin" "$(code "$BASE/users" -H "authorization: Bearer $TOKEN")" 403
ADMIN=$(curl -s -X POST "$BASE/users/login" -H "$J" \
  -d '{"email":"admin@e2e.dev","password":"admin-password-123"}' | token_of)
expect "users list as admin" \
  "$(curl -s "$BASE/users" -H "authorization: Bearer $ADMIN" | json "d['total']")" 2
expect "wrong password" "$(code -X POST "$BASE/users/login" -H "$J" \
  -d '{"email":"ada@e2e.dev","password":"wrong-password"}')" 401
expect "duplicate email" "$(code -X POST "$BASE/users/register" -H "$J" \
  -d '{"name":"B","email":"ada@e2e.dev","password":"longenough","is_admin":false}')" 409
AUTHOR=$(curl -s "$BASE/users/me" -H "authorization: Bearer $TOKEN" | json "d['id']")
AUTH="authorization: Bearer $TOKEN"
post_body() { printf '{"title":"%s","published":%s,"author_id":"%s"}' "$1" "$2" "$AUTHOR"; }
for i in 1 2 3; do
  BODY=$(post_body "Post $i" true)
  expect "create post $i" "$(code -X POST "$BASE/posts" -H "$AUTH" -H "$J" -d "$BODY")" 201
done
expect "unknown author (foreign key)" "$(code -X POST "$BASE/posts" -H "$AUTH" -H "$J" \
  -d '{"title":"x","published":true,"author_id":"00000000-0000-0000-0000-000000000000"}')" 409
BODY=$(post_body "" true)
expect "empty title (validation)" "$(code -X POST "$BASE/posts" -H "$AUTH" -H "$J" -d "$BODY")" 422
# Created a moment apart, so the default order must be newest first (ADR-022). A timestamp
# without sub-second precision would tie them and fall back to the random id.
expect "newest first" \
  "$(curl -s "$BASE/posts" | json "' | '.join(p['title'] for p in d['items'])")" \
  "Post 3 | Post 2 | Post 1"
expect "page 1 of 2 per page (items, total)" \
  "$(curl -s "$BASE/posts?per_page=2" | json "len(d['items']), d['total']")" "2 3"
expect "page 2 of 2 per page" "$(curl -s "$BASE/posts?page=2&per_page=2" | json "len(d['items'])")" 1
expect "per_page over the maximum" "$(code "$BASE/posts?per_page=101")" 422
read -r POST TITLE < <(curl -s "$BASE/posts?per_page=1" | json "d['items'][0]['id'], d['items'][0]['title']")
expect "show" "$(curl -s "$BASE/posts/$POST" | json "d['title']")" "$TITLE"
BODY=$(post_body Edited false)
expect "update" "$(curl -s -X PUT "$BASE/posts/$POST" -H "$AUTH" -H "$J" -d "$BODY" \
  | json "d['title']")" Edited
expect "delete" "$(code -X DELETE "$BASE/posts/$POST" -H "$AUTH")" 204
expect "show after delete" "$(code "$BASE/posts/$POST")" 404
expect "author still referenced (foreign key)" \
  "$(code -X DELETE "$BASE/users/$AUTHOR" -H "authorization: Bearer $ADMIN")" 409
stop_app

step "repeat generation: hand edits and migrations survive"
echo '// e2e: hand edit' >>src/app/posts/service.rs
BEFORE=$(migrations)
"$GIZE" make crud Post title:String published:bool author:belongs_to:users >/dev/null
expect "no duplicate CREATE TABLE migration" "$(migrations)" "$BEFORE"
grep -q 'e2e: hand edit' src/app/posts/service.rs || fail "make crud overwrote a hand edit"
echo "   ok  hand edit kept by make crud"

step "evolve the schema: add posts.views"
python3 - <<'EOF'
import re, pathlib
p = pathlib.Path("gize.toml"); s = p.read_text()
s, n = re.subn(r'(name = "posts"\nfields = \[)', r'\1"views:i32", ', s)
assert n == 1, s
p.write_text(s)
EOF
"$GIZE" sync >/dev/null
grep -q 'e2e: hand edit' src/app/posts/service.rs || fail "sync without --force overwrote a hand edit"
grep -q 'views' src/app/posts/model.rs && fail "sync without --force rewrote a drifted file"
echo "   ok  sync reports drift and keeps hand edits"
"$GIZE" sync --force >/dev/null
grep -q 'pub views: i32' src/app/posts/model.rs || fail "sync --force did not add the field"
echo "   ok  sync --force regenerates the module"

BEFORE=$(migrations)
"$GIZE" make migration >/dev/null
expect "model diff writes one ALTER migration" "$(migrations)" "$((BEFORE + 1))"
"$GIZE" make migration >/dev/null
expect "repeating the diff writes nothing" "$(migrations)" "$((BEFORE + 1))"
"$GIZE" migrate >/dev/null

# The added column is nullable with a backfill TODO (ADR-011). Backfill it and, where the
# database can do it in place, make it NOT NULL: the documented manual-migration flow.
"$GIZE" make migration backfill_post_views >/dev/null
BACKFILL=$(ls migrations/*_backfill_post_views.sql)
{
  echo "UPDATE posts SET views = 0 WHERE views IS NULL;"
  case "$DATABASE" in
    postgres) echo "ALTER TABLE posts ALTER COLUMN views SET NOT NULL;" ;;
    mysql) echo "ALTER TABLE posts MODIFY COLUMN views INT NOT NULL;" ;;
    sqlite) : ;; # SQLite needs a table rebuild for NOT NULL; the backfill alone is enough here.
  esac
} >>"$BACKFILL"
"$GIZE" migrate >/dev/null
OUT=$("$GIZE" make migration)
if [ "$DATABASE" != sqlite ]; then
  case "$OUT" in *"allows NULL"*) fail "the backfill was not recognized: $OUT" ;; esac
  echo "   ok  replay sees the column as NOT NULL"
fi
expect "no migration after the backfill" "$(migrations)" "$((BEFORE + 2))"

step "serve the evolved app"
start_app
expect "old rows read back with the backfilled value" \
  "$(curl -s "$BASE/posts" | json "sorted({p['views'] for p in d['items']})")" "[0]"
BODY=$(printf '{"title":"Viewed","published":true,"author_id":"%s","views":7}' "$AUTHOR")
NEW=$(curl -s -X POST "$BASE/posts" -H "$AUTH" -H "$J" -d "$BODY" | json "d['id']")
expect "new field round-trips" "$(curl -s "$BASE/posts/$NEW" | json "d['views']")" 7
expect "total after evolution" "$(curl -s "$BASE/posts" | json "d['total']")" 3
stop_app

echo "e2e OK ($DATABASE): routes, auth, pagination, constraints, repeat generation and schema evolution"

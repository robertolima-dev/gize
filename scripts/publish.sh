#!/usr/bin/env bash
# Publish the workspace crates to crates.io in dependency order, skipping every crate whose
# version is already in the index. A release that failed halfway (a network error, a rejected
# crate) is finished by running this again: the crates that made it are skipped instead of
# failing with "already exists".
#
# Usage: CARGO_REGISTRY_TOKEN=... scripts/publish.sh
#        DRY_RUN=1 scripts/publish.sh   # print what would be published; needs no token
#
# On a tag push (GITHUB_REF_NAME=vX.Y.Z) the tag must match the workspace version.
set -euo pipefail

# Dependencies before dependents: gize-generator needs core, templates, openapi and admin;
# the CLI needs core, generator and db. gize-auth and the examples are not published.
CRATES=(gize-core gize-templates gize-openapi gize-db gize-admin gize-generator gize-macros gize-testing gize)

VERSION="$(cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "gize") | .version')"
[ -n "$VERSION" ] || { echo "could not read the workspace version" >&2; exit 1; }

if [[ "${GITHUB_REF_NAME:-}" == v* && "${GITHUB_REF_NAME#v}" != "$VERSION" ]]; then
  echo "tag $GITHUB_REF_NAME does not match the workspace version $VERSION" >&2
  exit 1
fi

# The sparse index keeps one file per crate, one JSON line per published version, under
# <first two letters>/<next two>/<name> (every name here has at least 4 characters).
published() {
  local name="$1" body
  body="$(curl -sf "https://index.crates.io/${name:0:2}/${name:2:2}/$name")" || return 1
  grep -q "\"vers\":\"$VERSION\"" <<<"$body"
}

for crate in "${CRATES[@]}"; do
  if published "$crate"; then
    echo "skip     $crate $VERSION (already on crates.io)"
  elif [ -n "${DRY_RUN:-}" ]; then
    echo "publish  $crate $VERSION (dry run)"
  else
    echo "publish  $crate $VERSION"
    # cargo waits until the crate is in the index, so the next dependent resolves it.
    cargo publish -p "$crate" --token "${CARGO_REGISTRY_TOKEN:?set CARGO_REGISTRY_TOKEN}"
  fi
done

#!/usr/bin/env bash
set -euo pipefail

usage="usage: $0 [tag]"
[[ $# -le 1 ]] || { echo "$usage" >&2; exit 2; }

cargo_toml=${CARGO_TOML:-Cargo.toml}
manifest=${MANIFEST:-cornercase-plugin.toml}

first_version() {
  awk -F'"' -v section="$2" '
    /^\[/ { inside = ($0 == section) }
    inside && /^version *=/ { print $2; exit }
  ' "$1"
}

app=$(first_version "$cargo_toml" "[package]")
plugin=$(awk -F'"' '/^\[/ { exit } /^version *=/ { print $2; exit }' "$manifest")

fail() { echo "error: $*" >&2; exit 1; }

[[ -n $app ]] || fail "no version in [package] of $cargo_toml"
[[ -n $plugin ]] || fail "no version at the top of $manifest"
[[ $app == "$plugin" ]] || fail "$cargo_toml has version $app but $manifest has $plugin"

if [[ $# -eq 1 && $1 != "v$app" ]]; then
  fail "the tag $1 does not match version $app (expected v$app)"
fi

stale=$(awk -v version="$app" '
  /^\[/ { inside = ($0 ~ /^\[download\./) }
  inside && /^url *=/ && index($0, "/releases/download/v" version "/") == 0 { print }
' "$manifest")
[[ -z $stale ]] || fail "$manifest has downloads for another version than $app; remove its [download.*] tables when bumping: $stale"

echo "version $app matches in $cargo_toml and $manifest"

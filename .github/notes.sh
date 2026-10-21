#!/usr/bin/env bash
set -euo pipefail

usage="usage: $0 <tag>"
[[ $# -eq 1 ]] || { echo "$usage" >&2; exit 2; }

changelog=${CHANGELOG:-CHANGELOG.md}
version=${1#v}

notes=$(awk -v head="## $version" '
  $0 == head { inside = 1; next }
  inside && /^## / { exit }
  inside { print }
' "$changelog")

[[ -n ${notes//[[:space:]]/} ]] || { echo "error: $changelog has no section ## $version" >&2; exit 1; }
printf '%s\n' "$notes"

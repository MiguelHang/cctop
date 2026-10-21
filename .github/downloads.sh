#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 3 ]] || { echo "usage: $0 <owner/repo> <tag> <dir with cctop-<target>.tar.gz.sha256>" >&2; exit 2; }
repo=$1
tag=$2
dir=$3
manifest=${MANIFEST:-cornercase-plugin.toml}
targets=(x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin)

tables=""
for target in "${targets[@]}"; do
  file=$dir/cctop-$target.tar.gz.sha256
  [[ -f $file ]] || { echo "error: $file is missing" >&2; exit 1; }
  sha=$(awk '{ print $1; exit }' "$file")
  [[ $sha =~ ^[0-9a-f]{64}$ ]] || { echo "error: $file does not hold a sha256" >&2; exit 1; }
  tables+=$'\n'"[download.$target]"$'\n'
  tables+="url = \"https://github.com/$repo/releases/download/$tag/cctop-$target.tar.gz\""$'\n'
  tables+="sha256 = \"$sha\""$'\n'
done

kept=$(awk '
  /^\[/ { inside = ($0 ~ /^\[download\./) }
  !inside { print }
' "$manifest")
printf '%s\n%s' "$kept" "$tables" >"$manifest"

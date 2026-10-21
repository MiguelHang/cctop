#!/usr/bin/env bash
set -euo pipefail

[[ $# -eq 2 ]] || { echo "usage: $0 <target> <out dir>" >&2; exit 2; }
target=$1
out=$2

binary=target/$target/release/cctop
[[ -x $binary ]] || { echo "error: $binary is missing or not executable" >&2; exit 1; }

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/target/release" "$out"
cp "$binary" "$stage/target/release/cctop"
chmod 0755 "$stage/target/release/cctop"

archive=cctop-$target.tar.gz
COPYFILE_DISABLE=1 tar --format=ustar -czf "$out/$archive" -C "$stage" target

if command -v sha256sum >/dev/null; then
  (cd "$out" && sha256sum "$archive" >"$archive.sha256")
else
  (cd "$out" && shasum -a 256 "$archive" >"$archive.sha256")
fi
cat "$out/$archive.sha256"

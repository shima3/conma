#!/bin/bash
# file.se を includer と resolver に通し、dir/file.ast.se と dir/file.module.se を作る（dir の既定は /tmp）。
#   test/resolver.sh [path/]file.se [dir]
# SInfo のファイル名は、リポジトリの上のディレクトリからの相対パス（"conma/..."）にする
set -o pipefail
root="$(cd "${0%/*}/.."; pwd)"
bin="$root/bin"
base="$(basename "${1%.se}")"
out="${2:-/tmp}"
ast="$out/${base}.ast.se"
$bin/includer --include "$root/include" "$1" | sed -e 's/"\/.*\/conma/"conma/' > "$ast" &&
$bin/resolver < "$ast" > "$out/${base}.module.se"

#!/bin/bash
# file.se を includer と resolver に通し、dir/file.ast.se と dir/file.module.se を作る（dir の既定は /tmp）。
#   test/resolver.sh [path/]file.se [dir]
# SInfo のファイル名は、リポジトリの上のディレクトリからの相対パス（"conma/..."）にする
# 環境変数 CONMA_REFERENCE を空でない値にすると、標準ライブラリの参照版で作る（reference/ を include/ より先に探す。
# reference/README-ja.md）
set -o pipefail
root="$(cd "${0%/*}/.."; pwd)"
bin="$root/bin"
base="$(basename "${1%.se}")"
out="${2:-/tmp}"
ast="$out/${base}.ast.se"
if [ -n "$CONMA_REFERENCE" ]; then
  incs=(--include "$root/reference" --include "$root/include")
else
  incs=(--include "$root/include")
fi
$bin/includer "${incs[@]}" "$1" | sed -e 's/"\/.*\/conma/"conma/' > "$ast" &&
$bin/resolver < "$ast" > "$out/${base}.module.se"

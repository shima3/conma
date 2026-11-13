#!/bin/bash
# file.se を includer と resolver に通し、dir/file.ast.se と dir/file.module.se を作る（dir の既定は /tmp）。
#   test/resolver.sh [path/]file.se [dir]
# SInfo のファイル名は、リポジトリの上のディレクトリからの相対パス（"conma/..."）にする
# 簡潔版 ConMa のモジュールを作る（include/ref を include より先に探す。__TRUE__ と __FALSE__ は include/ref/pure.se の
# 定義になる）。test/ のログは、Python VM（簡潔版）と同じトレースとステップ数を、どの VM でも比べるため。
# 環境変数 CONMA_REFERENCE を空でない値にすると、標準ライブラリの参照版で作る（reference/ も include/ より先に探す。
# reference/README-ja.md）。環境変数 CONMA_FAST を空でない値にすると、高速版で作る（include/fast を先に探す）
set -o pipefail
root="$(cd "${0%/*}/.."; pwd)"
bin="$root/bin"
base="$(basename "${1%.se}")"
out="${2:-/tmp}"
ast="$out/${base}.ast.se"
if [ -n "$CONMA_FAST" ]; then
  incs=(--include "$root/include/fast" --include "$root/include")
elif [ -n "$CONMA_REFERENCE" ]; then
  incs=(--include "$root/include/ref" --include "$root/reference" --include "$root/include")
else
  incs=(--include "$root/include/ref" --include "$root/include")
fi
if [ -n "$CONMA_FAST" ]; then
  $bin/includer "${incs[@]}" "$1" | sed -e 's/"\/.*\/conma/"conma/' > "$ast" &&
  $bin/resolver < "$ast" > "$out/${base}.module.se"
else
  # 簡潔版：include/ref にない名前（ファイルが pure.se を include しないときの __if__ など）を組み込みとして使うと、
  # フロントエンドが (Uses ...) を書くので、bin/concise で代わりの定義を入れて、簡潔版のモジュールにする
  $bin/includer "${incs[@]}" "$1" | sed -e 's/"\/.*\/conma/"conma/' > "$ast" &&
  $bin/resolver < "$ast" | $bin/concise > "$out/${base}.module.se"
fi

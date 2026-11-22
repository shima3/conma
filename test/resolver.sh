#!/bin/bash
# file.se を includer と resolver に通し、dir/file.ast.se と dir/file.module.se を作る（dir の既定は /tmp）。
#   test/resolver.sh [path/]file.se [dir]
# SInfo のファイル名は、リポジトリの上のディレクトリからの相対パス（"conma/..."）にする
# コア ConMa のモジュールを作る（include/core を include より先に探す。__TRUE__ と __FALSE__ は include/core/pure.se の
# 定義になる）。test/ のログは、Python VM（コア ConMa）と同じトレースとステップ数を、どの VM でも比べるため。
# 環境変数 CONMA_REFERENCE を空でない値にすると、標準ライブラリの参照版で作る（reference/ も include/ より先に探す。
# reference/README-ja.md）。環境変数 CONMA_EXT を空でない値にすると、拡張 ConMa で作る（include/ext を先に探す）
set -o pipefail
root="$(cd "${0%/*}/.."; pwd)"
bin="$root/bin"
base="$(basename "${1%.se}")"
out="${2:-/tmp}"
ast="$out/${base}.ast.se"
if [ -n "$CONMA_EXT" ]; then
  incs=(--include "$root/include/ext" --include "$root/include")
elif [ -n "$CONMA_REFERENCE" ]; then
  incs=(--include "$root/include/core" --include "$root/reference" --include "$root/include")
else
  incs=(--include "$root/include/core" --include "$root/include")
fi
if [ -n "$CONMA_EXT" ]; then
  $bin/includer "${incs[@]}" "$1" | sed -e 's/"\([A-Za-z]:\)\{0,1\}\/.*\/conma/"conma/' > "$ast" &&
  $bin/resolver < "$ast" > "$out/${base}.module.se"
else
  # コア ConMa：include/core にない名前（ファイルが pure.se を include しないときの __if__ など）を組み込みとして使うと、
  # フロントエンドが (Uses ...) を書くので、bin/to-core で代わりの定義を入れて、コア ConMa のモジュールにする
  $bin/includer "${incs[@]}" "$1" | sed -e 's/"\([A-Za-z]:\)\{0,1\}\/.*\/conma/"conma/' > "$ast" &&
  $bin/resolver < "$ast" | $bin/to-core > "$out/${base}.module.se"
fi

#!/bin/bash
# SJS：算術評価 (( )) と算術置換 $(( ))（consh/SAFEJS-PLAN-ja.md の 2）。wasm 版の VM（Node.js）で ConMa 版の ConSh を動かす
cd ${0%/*}
export CONMA_VM=wasm
export CONSH_IMPL=conma
# 算術評価だけを使う（js は使えない）
export CONSH_ENABLE_JS=sjs
. ../../../test/common.sh
# case1 は、引数のディレクトリ（一時ディレクトリ）にファイルを書く
for c in case1 case2 case3
do
  run $c $c.sh "$tmp" < /dev/null
done
# 対話モード（続きの行、エラーで止めたコマンド行）
run case4 < case4.in
# CONSH_ENABLE_JS が js なら js も使え、ないかほかの値なら js は使えない（(( )) は段階 B で動くので、いつも使える）
(export CONSH_ENABLE_JS=js; run case5js case5.sh < /dev/null)
(unset CONSH_ENABLE_JS; run case5off case5.sh < /dev/null)
(export CONSH_ENABLE_JS=yes; run case5bad case5.sh < /dev/null)
# 語の中の $(( )) のエラーは、スクリプトを終了ステータス 2 で終える
run case6 case6.sh < /dev/null
# 段階 A2
run case7 case7.sh < /dev/null
run case8 case8.sh < /dev/null
run case9 case9.sh < /dev/null

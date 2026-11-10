#!/bin/bash
# JS の文とシェルの変数の JS の値（consh/js/PLAN-ja.md の 2.5）。wasm 版の VM（Node.js）で ConMa 版の ConSh を動かす
cd ${0%/*}
export CONMA_VM=wasm
export CONSH_IMPL=conma
# js と JS の文を使う（consh/js/README-ja.md）
export CONSH_ENABLE_JS=js
. ../../../test/common.sh
for c in case1 case2 case3 case4 case6 case7
do
  run $c $c.sh < /dev/null
done
# 対話モード（続きの行、今定義されている関数）
run case5 < case5.in

#!/bin/bash
# シェルの変数の JS の値（consh/js/PLAN-ja.md の 2.5 の 4。JS の文は段階 A3 で消した）。wasm 版の VM（Node.js）で ConMa 版の ConSh を動かす
cd ${0%/*}
export CONMA_VM=wasm
export CONSH_IMPL=conma
# js と算術評価を使う（consh/js/README-ja.md）
export CONSH_ENABLE_JS=js
. ../../../test/common.sh
for c in case1 case2 case3
do
  run $c $c.sh < /dev/null
done

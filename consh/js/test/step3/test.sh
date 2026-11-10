#!/bin/bash
# 段階 6：イテレータの結果を1つずつ出す（consh/js/PLAN-ja.md の 2.1 の 5 の最後と、6 の遅延の LazyList）。
# wasm 版の VM（Node.js）で ConMa 版の ConSh を動かす
cd ${0%/*}
export CONMA_VM=wasm
export CONSH_IMPL=conma
# js と JS の文を使う（consh/js/README-ja.md）
export CONSH_ENABLE_JS=1
. ../../../test/common.sh
for c in case1 case2 case3
do
  run $c $c.sh < /dev/null
done

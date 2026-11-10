#!/bin/bash
# 段階 5：js の stdin と console（consh/js/PLAN-ja.md の 2.1 の 7、8）。wasm 版の VM（Node.js）で ConMa 版の ConSh を動かす
cd ${0%/*}
export CONMA_VM=wasm
export CONSH_IMPL=conma
# js と JS の文を使う（consh/js/README-ja.md）
export CONSH_ENABLE_JS=js
. ../../../test/common.sh
for c in case1 case2 case3
do
  run $c $c.sh "$tmp" < /dev/null
done

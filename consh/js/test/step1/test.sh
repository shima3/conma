#!/bin/bash
# 段階 4：組み込みコマンド js（consh/js/PLAN-ja.md の 2.1）。wasm 版の VM（vm-wasm.sh。Node.js）で ConMa 版の ConSh を動かす。
# consh/js/build.sh で wasm を作っておく
cd ${0%/*}
export CONMA_VM=wasm
export CONSH_IMPL=conma
. ../../../test/common.sh
for c in case1 case2 case3 case4
do
  run $c $c.sh < /dev/null
done

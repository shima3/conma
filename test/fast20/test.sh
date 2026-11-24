#!/bin/bash
# __JS_fn_compile__ と __JS_fn_call__（docs/dev/CONJS-HOST-PLAN.md の段階 H2。vm/src/prim/jsfn.rs）：VM の値を写して、ホストの
# JavaScript の関数を呼び、変えたものを書き戻す：数、文字列、真偽、LList、Dict（同じものと輪を保つ）、Ref、ハンドル（Closure を
# 呼び返す）、-0、NaN、無限大、throw、escape、ないもの、送れない値、構文エラー、SJS の配列の形。ログは、wasm 版の VM が
# case1.wasm.log、ほかの VM が case1.log（JavaScript がない）
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
vm="${CONMA_VM:-python}"
[ "$vm" = transpiled ] && vm=rust
for c in case1
do
  log=$c.$vm.log
  [ -f $log ] || log=$c.log
  PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log
done

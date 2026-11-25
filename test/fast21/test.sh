#!/bin/bash
# __JS_fn_resume__ と、写したものを合わせること（docs/dev/CONJS-HOST-PLAN.md の段階 H2b。vm/src/prim/jsfn.rs）：ホストの
# ジェネレータが yield で VM に戻り、続ける。yield の間に VM が変えたものを、ホストは続けた後に見る。yield に例外を投げる。
# yield で出したオブジェクトを VM が変える。終わったジェネレータ。呼び返しに渡したオブジェクトは同じ Dict（case2）。
# ログは、wasm 版の VM が case*.wasm.log、ほかの VM が case*.log（JavaScript がない）
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
vm="${CONMA_VM:-python}"
[ "$vm" = transpiled ] && vm=rust
for c in case1 case2
do
  log=$c.$vm.log
  [ -f $log ] || log=$c.log
  PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log
done

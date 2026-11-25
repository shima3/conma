#!/bin/bash
# 呼び返し（docs/dev/CONJS-HOST-PLAN.md の段階 H1）：ホストの関数が、this.callVm で ConMa の大域の関数を呼び返す
# （vm/wasm/conma.js、vm/wasm/src/lib.rs の conma_callback、vm/src/prim/host.rs の callback）。呼び返しは、入れ子の
# 実行で最後まで動く：数、Dict と LList、呼び返しの中からもう一度ホスト（2 段の入れ子）、エラー、待つもの（escape）、
# ない関数、PDict の受け渡し。ログは、wasm 版の VM が case1.wasm.log、ほかの VM が case1.log（その関数がない）
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
vm="${CONMA_VM:-python}"
[ "$vm" = transpiled ] && vm=rust
for c in case1
do
  log=$c.$vm.log
  [ -f $log ] || log=$c.log
  CONMA_HOST="$(pwd)/host.js" PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log
done

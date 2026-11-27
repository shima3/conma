#!/bin/bash
# 値を参照で渡すこと（docs/dev/CONJS-SHELL-PLAN.md の段階 S0。vm/src/prim/jsfn.rs）：VM がホストのオブジェクトを
# 持ち、呼び出しを越えて同じオブジェクトとして渡す（case1）。ホストが VM の値を持ち続け、後の呼び出しで同じ値として返す（case2）。
#
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

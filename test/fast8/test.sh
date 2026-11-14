#!/bin/bash
# 数の定数の畳み込みと、変数の多い連鎖のフレーム（test.se）。Rust VM では、畳み込みなし（CONMA_NO_FOLD=1）と
# フレームなし（CONMA_FRAME_MIN=）でも、同じログになる
. "$(dirname "$0")/../common.sh"
resolve test.se
envs=("")
[ "$CONMA_VM" = rust ] && envs+=("CONMA_NO_FOLD=1" "CONMA_FRAME_MIN=")
for e in "${envs[@]}"
do
  for c in case1 case2 case3 case4 case5
  do
    env $e PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $c.log $tmp/$c.log || echo "$c ${e:-(default)}"
  done
done

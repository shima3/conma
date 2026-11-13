#!/bin/bash
# 各 case を、組み込みの __Cont_get__（test.se）と ConMa 版（conma.se）の両方で実行して比べる
. "$(dirname "$0")/../common.sh"
# どちらも高速版 ConMa で作る（include/fast を先に探す）。test.se は組み込み（(Uses "control")）を、conma.se は
# include/ref/control.se の定義を使う。Python VM（簡潔版）では、test/vm.sh が bin/concise で直すので、どちらも定義になる
CONMA_FAST=1 resolve test.se conma.se
for c in case1 case2 case3 case4 case5 case6
do
  for m in test conma
  do
    PYTHONUNBUFFERED=1 ../vm.sh $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $c.log $tmp/$c.log
  done
done

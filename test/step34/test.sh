#!/bin/bash
# 各 case を、組み込みの __Cont_get__（test.se）と ConMa 版（conma.se）の両方で実行して比べる
. "$(dirname "$0")/../common.sh"
resolve test.se conma.se
for c in case1 case2 case3 case4 case5 case6
do
  for m in test conma
  do
    PYTHONUNBUFFERED=1 ../vm.sh $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $c.log $tmp/$c.log
  done
done

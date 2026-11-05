#!/bin/bash
# 各 case を、組み込みの __Dict_copy__ など（test.se）と ConMa 版（conma.se、data.se）の両方で実行して比べる
cd ${0%/*}
if ! ../resolver.sh test.se || ! ../resolver.sh conma.se
then exit 1
fi
for c in case1 case2 case3 case4
do
  for m in test conma
  do
    PYTHONUNBUFFERED=1 ../vm.sh /tmp/$m.module.se $c < /dev/null > /tmp/$c.log 2>&1
    echo "exit: $?" >> /tmp/$c.log
    diff $c.log /tmp/$c.log
  done
done

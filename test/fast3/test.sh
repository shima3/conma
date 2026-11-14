#!/bin/bash
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2 case3 case4 case5
do
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done
# Rust VM が、節を関数で書いた条件分岐の Body に印を付けること（vm/src/module.rs の if_site）。
# 印が付かなくてもステップの番号は同じなので、ここで数を確かめる
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  CONMA_FAST_REPORT=1 "$conma" $tmp/test.module.se case1 2>&1 | grep "^fast conditionals" > $tmp/report.log
  diff report.log $tmp/report.log
fi

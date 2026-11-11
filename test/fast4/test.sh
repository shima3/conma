#!/bin/bash
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2 case3 case4 case5
do
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done
# Rust VM が、これらのプリミティブを LCont を関数で書いて適用する Body に印を付けること（vm/src/module.rs の prim_site）
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  CONMA_FAST_REPORT=1 "$conma" $tmp/test.module.se case3 2>&1 | grep "^fast primitives" > $tmp/report.log
  diff report.log $tmp/report.log
fi

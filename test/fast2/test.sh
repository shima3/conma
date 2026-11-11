#!/bin/bash
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2 case3 case4 case5
do
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done
# Rust VM が、include/util.se と reference/util.se の __LazyList_force__ に印を付けること
# （定義を書き換えると印が付かなくなり、ここで違いが出る。vm/src/module.rs の mark_fast）
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  for dir in include reference
  do
    echo '(include "util.se") (define main ,() __OS_exit_normal__)' > $tmp/report.se
    "$conma" build --include "$root/$dir" $tmp/report.se > $tmp/report.module.se &&
      CONMA_FAST_REPORT=1 "$conma" $tmp/report.module.se main 2>&1 | grep LazyList | sed "s/^/$dir: /"
  done > $tmp/report.log
  diff report.log $tmp/report.log
fi

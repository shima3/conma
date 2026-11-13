#!/bin/bash
# Step41：高速版 ConMa の機能 "bitwise32" の代わりの定義（include/ref/bitwise32.se）。ログは test.js（Node.js）の値
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2 case3
do
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

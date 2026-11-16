#!/bin/bash
# Step47：拡張 ConMa の機能 "array" の代わりの定義（include/core/array.se。docs/dev/ARRAY-PLAN-ja.md）
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2 case3 case4 case5
do
  PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

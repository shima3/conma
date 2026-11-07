#!/bin/bash
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2 case3
do
  STEP37_DIR="$tmp" PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

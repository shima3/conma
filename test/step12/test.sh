#!/bin/bash
. "${0%/*}/../common.sh"
resolve test.se
for c in case1 case2 case3 case4 case5 case6 case7
do
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

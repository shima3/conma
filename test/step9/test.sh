#!/bin/bash
. "${0%/*}/../common.sh"
resolve test.se
for c in case1 case2 case3 case4 case5
do
  # case5 reads standard input; the other cases get an empty one
  if [ $c = case5 ]; then input=input.txt; else input=/dev/null; fi
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < $input > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

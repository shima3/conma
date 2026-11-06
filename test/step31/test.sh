#!/bin/bash
. "${0%/*}/../common.sh"
resolve test.se
for c in case1
do
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null 2>&1 > $tmp/$c.log
  echo "exit: ${PIPESTATUS[0]}" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

#!/bin/bash
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2
do
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

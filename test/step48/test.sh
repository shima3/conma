#!/bin/bash
# Step48：最後の適用が __VProc_wait__ か __Ref_wait_change__ の VProc が、起きたときに終わる
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2 case3
do
  PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

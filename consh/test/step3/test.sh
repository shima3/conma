#!/bin/bash
cd ${0%/*}
. ../common.sh
for c in case1 case2 case3 case5 case6 case7 case8 case9
do
  run $c $c.sh < /dev/null
done
d=$(mktemp -d /tmp/consh.step3.XXXXXX)
run case4 case4.sh $d < /dev/null
rm -rf $d

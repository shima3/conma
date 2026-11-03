#!/bin/bash
cd ${0%/*}
. ../common.sh
for c in case1 case2 case3 case5 case6 case7 case8 case9 case10 case12 case14 case15 case16 case17
do
  run $c $c.sh < /dev/null
done
d=$(mktemp -d /tmp/consh.step3.XXXXXX)
run case4 case4.sh $d < /dev/null
run case11 case11.sh $d < /dev/null
run case13 case13.sh $d < /dev/null
rm -rf $d

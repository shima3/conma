#!/bin/bash
cd ${0%/*}
. ../common.sh
for c in case1 case2 case3 case4 case5 case7 case8 case9 case10
do
  run $c $c.sh < /dev/null
done
run case6 case6.sh < case6.input

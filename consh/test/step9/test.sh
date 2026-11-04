#!/bin/bash
cd ${0%/*}
. ../common.sh
for c in case1 case2 case3 case4
do
  run $c $c.sh < /dev/null
done

#!/bin/bash
cd ${0%/*}
. ../common.sh
d=$(mktemp -d /tmp/consh.step10.XXXXXX)
run case1 case1.sh < /dev/null
run case2 case2.sh $d < /dev/null
run case3 case3.sh $d < /dev/null
rm -rf $d

#!/bin/bash
cd ${0%/*}
. ../common.sh
# js（consh/js/PLAN-ja.md の 2.1）を、JavaScript のない版で
for c in case1 case2 case3
do
  run $c $c.sh < /dev/null
done

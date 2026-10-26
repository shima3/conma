#!/bin/bash
cd ${0%/*}
if ! ../resolver.sh test.se
then exit 1
fi
for c in case1 case2 case3 case4 case5 case6 case7 case8 case9
do
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py /tmp/test.module.se $c > /tmp/$c.log 2>&1
  echo "exit: $?" >> /tmp/$c.log
  diff $c.log /tmp/$c.log
done

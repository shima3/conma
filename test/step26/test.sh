#!/bin/bash
cd ${0%/*}
if ! ../resolver.sh test.se
then exit 1
fi
for c in case1 case2 case3 case4 case5
do
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py /tmp/test.module.se $c < /dev/null 2>&1 > /tmp/$c.log
  echo "exit: ${PIPESTATUS[0]}" >> /tmp/$c.log
  diff $c.log /tmp/$c.log
done

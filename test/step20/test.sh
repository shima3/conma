#!/bin/bash
cd ${0%/*}
if ! ../resolver.sh test.se
then exit 1
fi
for c in case1 case2 case3 case4 case5 case6 case7 case8 case9 case10 case11 case12 case13 case14 case15 case16 case17
do
  # case6 reads standard input; the other cases get an empty one
  if [ $c = case6 ]; then input=input.txt; else input=/dev/null; fi
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py /tmp/test.module.se $c < $input > /tmp/$c.log 2>&1
  echo "exit: $?" >> /tmp/$c.log
  diff $c.log /tmp/$c.log
done

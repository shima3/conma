#!/bin/bash
cd ${0%/*}
for f in test.se worker.se
do
  if ! ../resolver.sh $f
  then exit 1
  fi
done
run() {
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py /tmp/test.module.se $1 < /dev/null > /tmp/$1.log 2>&1
  echo "exit: $?" >> /tmp/$1.log
  diff $1.log /tmp/$1.log
}
# case2 and case3 wait for 0.2 and 1 second. The output and the number of
# steps ([done] line of --trace) must be the same: waiting uses no steps.
steps() {
  rm -f /tmp/$1.log
  for delay in 0.2 1
  do
    echo "delay: $delay" >> /tmp/$1.log
    DELAY=$delay PYTHONUNBUFFERED=1 python3 ../vproc_run.py --trace /tmp/test.module.se $1 < /dev/null 2>/tmp/$1.trace >> /tmp/$1.log
    echo "exit: $?" >> /tmp/$1.log
    grep '^\[done\]' /tmp/$1.trace >> /tmp/$1.log
  done
  diff $1.log /tmp/$1.log
}
run case1
steps case2
steps case3
for c in case4 case5 case6 case7
do
  run $c
done

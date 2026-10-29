#!/bin/bash
cd ${0%/*}
if ! ../resolver.sh main.se
then exit 1
fi
run() {
  local c=$1; shift
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py --main /tmp/main.module.se "$@" > /tmp/$c.log 2>&1
  echo "exit: $?" >> /tmp/$c.log
  diff $c.log /tmp/$c.log
}
run case1 read < input.txt
run case2 read < /dev/null
run case3 child < input.txt
# an endless standard input: only the lines that are read are read from it
yes "y" | run case4 first3
# case5 and case6 count the line cells that stay alive while 200 lines are read (memcheck.py).
# case5 reads them with __stdin_read__: the lines already read are collected.
# case6 keeps the first stdin in PDict, so none is collected (a check that memcheck.py finds them)
for i in $(seq 1 200); do echo "line $i"; done > /tmp/step21.lines.txt
memcheck() {
  local c=$1; shift
  python3 memcheck.py /tmp/main.module.se "$@" < /tmp/step21.lines.txt > /tmp/$c.log 2>&1
  echo "exit: $?" >> /tmp/$c.log
  diff $c.log /tmp/$c.log
}
memcheck case5 drain
memcheck case6 drainKeep

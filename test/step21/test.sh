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

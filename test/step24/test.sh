#!/bin/bash
. "${0%/*}/../common.sh"
resolve test.se
# The current directory is printed as <dir>, so that the logs do not depend on where the repository is
dir="$(pwd -P)"
for c in case1 case2 case3 case4 case5
do
  rm -rf work
  mkdir -p work/a
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null 2>&1 | sed -e "s|$dir|<dir>|g" > $tmp/$c.log
  echo "exit: ${PIPESTATUS[0]}" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done
rm -rf work

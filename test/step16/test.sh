#!/bin/bash
cd ${0%/*}
if ! ../resolver.sh test.se
then exit 1
fi
# The current directory is printed as <dir>, so that the logs do not depend on where the repository is
dir="$(pwd -P)"
for c in case1 case2 case3 case4 case5 case6 case7
do
  # Every case starts with a work directory that has sub/ and a link to the test directory
  rm -rf work
  mkdir work work/sub
  ln -s .. work/up
  PYTHONUNBUFFERED=1 python3 ../vproc_run.py /tmp/test.module.se $c < /dev/null 2>&1 | sed -e "s|$dir|<dir>|g" > /tmp/$c.log
  echo "exit: ${PIPESTATUS[0]}" >> /tmp/$c.log
  diff $c.log /tmp/$c.log
done
rm -rf work

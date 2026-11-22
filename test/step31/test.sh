#!/bin/bash
. "$(dirname "$0")/../common.sh"
resolve test.se
# Git Bash（Windows）の kill -INT は、Windows のプログラム（VM）に届かず、待ち続ける（docs/dev/TESTS-ja.md の既知の違い）
if command -v cygpath > /dev/null; then
  echo "step31: not run on Windows: kill -INT does not reach the VM"
  exit 0
fi
for c in case1
do
  PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null 2>&1 > $tmp/$c.log
  echo "exit: ${PIPESTATUS[0]}" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

#!/bin/bash
# Step46：文字列の位置と長さ（docs/dev/STRING-INDEX-PLAN-ja.md の S3）。境目の値で、結果とエラーが Python 版の VM と同じこと
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1
do
  PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

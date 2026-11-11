#!/bin/bash
. "$(dirname "$0")/../common.sh"
resolve test.se
for c in case1 case2 case3 case4 case5 case6 case7 case8 case9 case10 case11
do
  # VProc 1 以外のエラーの、全体で数えたステップの番号は、VProc を切り替える時によるので比べない
  # （高速版の VM はステップをまとめて切り替えが変わる。docs/FAST-PLAN-ja.md の 3.3）
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c 2>&1 | sed 's/^\(Error in VProc [2-9][0-9]*\) at step [0-9]*:/\1 at step N:/' > $tmp/$c.log
  echo "exit: ${PIPESTATUS[0]}" >> $tmp/$c.log
  diff $c.log $tmp/$c.log
done

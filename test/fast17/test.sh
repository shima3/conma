#!/bin/bash
# 言語キット（include/lang。docs/dev/PLATFORM-PLAN-ja.md の 4.3 節、段階 P4）の部品だけで作った、小さな電卓の言語 calc。
# calc.se（変換器）で caseN.calc を ConMa のソースにし、calc_rt.se（ランタイム）と一緒にモジュールにして動かす。
# 変換も実行も、選んだ VM（CONMA_VM）で行う。どの VM でも同じログ。キットの部品が SJS の名前を使わないこと、字句、構文、
# 優先順位の 2 項演算（Kit_parse_binary）、名前の有効範囲、SInfo の印とエラーの位置（Kit_err_pos、Kit_err_report）、
# キットのエラーのメッセージを確かめる
. "$(dirname "$0")/../common.sh"
resolve calc.se
cp calc_rt.se $tmp/
for f in case*.calc
do
  c=${f%.calc}
  PYTHONUNBUFFERED=1 ../../bin/vm --main $tmp/calc.module.se $f > $tmp/$c.se 2> $tmp/$c.log < /dev/null
  echo "translate exit: $?" >> $tmp/$c.log
  if grep -q '^translate exit: 0$' $tmp/$c.log; then
    "$test/resolver.sh" $tmp/$c.se $tmp > /dev/null || exit 1
    PYTHONUNBUFFERED=1 ../../bin/vm $tmp/$c.module.se main >> $tmp/$c.log 2>&1 < /dev/null
    echo "exit: $?" >> $tmp/$c.log
  fi
  diff $c.log $tmp/$c.log
done

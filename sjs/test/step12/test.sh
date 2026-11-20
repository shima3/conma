#!/bin/bash
# JSON.parse と配列の sort（docs/dev/PLATFORM-PLAN-ja.md の 4.5 節。段階 P5b で、ConJS と一緒に足した）：case1 のログは
# Node.js で同じプログラムを動かした結果と比べた。case2（キーが constructor のオブジェクト）は、読むと TypeError になる
# （JavaScript では読める）
cd ${0%/*}
for f in case*.safe.js
do
  c=${f%.safe.js}
  ../../sjs $f > ${TMPDIR:-/tmp}/sjs-test-$$.log 2>&1
  echo "exit: $?" >> ${TMPDIR:-/tmp}/sjs-test-$$.log
  diff $c.log ${TMPDIR:-/tmp}/sjs-test-$$.log
done
rm -f ${TMPDIR:-/tmp}/sjs-test-$$.log

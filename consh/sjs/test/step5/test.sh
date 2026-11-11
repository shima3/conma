#!/bin/bash
# SJS の変換器の段階 B5b（async、await、Promise、マイクロタスクの列）：SJS のプログラムを ConMa に変換して、Rust 版の VM で動かす。
# 段階 A には async がないので、ログは Node.js で同じプログラムを動かした結果と比べた（case3 の、受け手のない拒否のメッセージを除く。順序も同じ）
cd ${0%/*}
for f in case*.sjs
do
  c=${f%.sjs}
  ../../sjs $f > /tmp/sjs-test-$$.log 2>&1
  echo "exit: $?" >> /tmp/sjs-test-$$.log
  diff $c.log /tmp/sjs-test-$$.log
done
rm -f /tmp/sjs-test-$$.log

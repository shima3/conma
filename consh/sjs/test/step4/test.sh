#!/bin/bash
# SJS の変換器の段階 B5（ジェネレータ：function*、yield、yield*、next、return、throw、for...of、スプレッド、分割代入、関数の宣言）：SJS のプログラムを ConMa に変換して、Rust 版の VM で動かす。
# 段階 A にはジェネレータがないので、ログは Node.js で同じプログラムを動かした結果と比べた（配列の console.log の形と、エラーの行の前置きを除く）
cd ${0%/*}
for f in case*.sjs
do
  c=${f%.sjs}
  ../../sjs $f > /tmp/sjs-test-$$.log 2>&1
  echo "exit: $?" >> /tmp/sjs-test-$$.log
  diff $c.log /tmp/sjs-test-$$.log
done
rm -f /tmp/sjs-test-$$.log

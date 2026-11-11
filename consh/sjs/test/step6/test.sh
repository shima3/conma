#!/bin/bash
# SJS の変換器の段階 B の残り（コンマ演算子、配列の穴、for の分割代入、ラベル、?. の鎖、ジェネレータを閉じること）：SJS のプログラムを ConMa に変換して、Rust 版の VM で動かす。
# ログは Node.js で同じプログラムを動かした結果と比べた
cd ${0%/*}
for f in case*.sjs
do
  c=${f%.sjs}
  ../../sjs $f > /tmp/sjs-test-$$.log 2>&1
  echo "exit: $?" >> /tmp/sjs-test-$$.log
  diff $c.log /tmp/sjs-test-$$.log
done
rm -f /tmp/sjs-test-$$.log

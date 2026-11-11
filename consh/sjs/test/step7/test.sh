#!/bin/bash
# SJS の言語の残り（タグ付きテンプレート、配列の穴とメソッド、length への代入、let の重なり、async ジェネレータと for await）：
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

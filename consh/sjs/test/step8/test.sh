#!/bin/bash
# SJS の in 演算子、配列の添字でないメンバー、Object.keys などの配列と文字列、レビューで直したもの、配列の分割代入：
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

#!/bin/bash
# SJS の変換器の段階 B3（throw、try、catch、finally、実行時のエラーと位置。case5〜case9 は段階 B9 の位置。Node.js と同じ行と桁）：SJS のプログラムを ConMa に変換して、Rust 版の VM で動かす。
# ログは、段階 A（ホストの JavaScript）で同じプログラムを動かした結果と同じ（エラーの行の前置きを除く。../compare.sh で確かめる）
cd ${0%/*}
for f in case*.sjs
do
  c=${f%.sjs}
  ../../sjs $f > ${TMPDIR:-/tmp}/sjs-test-$$.log 2>&1
  echo "exit: $?" >> ${TMPDIR:-/tmp}/sjs-test-$$.log
  diff $c.log ${TMPDIR:-/tmp}/sjs-test-$$.log
done
rm -f ${TMPDIR:-/tmp}/sjs-test-$$.log

#!/bin/bash
cd ${0%/*}
. ../common.sh
require lsof
# REVIEW-ja.md の 2026-11-05 の指摘への対応
d=$(mktemp -d /tmp/consh.step14.XXXXXX)
for c in case1 case2 case3 case4 case5 case6
do
  run $c $c.sh $d < /dev/null
done
rm -rf $d
# case7：対話モードで、OS のパイプの要素が起動の前にエラーになってもプロンプトに戻る（レビュー 3）。
# 標準入力が端末でないときの Ctrl-C（シェルにだけ SIGINT が届く）で、実行中の外部コマンドも止める（レビュー 10）
run case7 < case7.input

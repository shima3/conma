#!/bin/bash
cd ${0%/*}
. ../common.sh
# case1 から case5 と case7：引数なしの consh は対話モード。caseN.input を標準入力から読む（プロンプトは標準エラー出力）
# case7：Ctrl-C（SIGINT）で、実行中のコマンドだけを止める（$? は 130）。& の sh がシェルに SIGINT を送る
for c in case1 case2 case3 case4 case5 case7
do
  run $c < $c.input
done
run case6 case6.sh < /dev/null
# case8：疑似端末の中で起動し、Ctrl-C で前景の sleep だけを止め、& で起動した sh は止めない（約 12 秒かかる）
python3 case8.py > $tmp/case8.log 2>&1
echo "exit: $?" >> $tmp/case8.log
diff case8.log $tmp/case8.log

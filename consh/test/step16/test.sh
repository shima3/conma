#!/bin/bash
cd ${0%/*}
. ../common.sh
# 段階 16：外部コマンド1つだけの $( ) を、チャネルではなく OS のパイプでつなぐ
for c in case1 case2
do
  run $c $c.sh < /dev/null
done
# case3：fd の上限を小さくして、パイプが閉じられることを確かめる
(ulimit -n 64; run case3 case3.sh < /dev/null)

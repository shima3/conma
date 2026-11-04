#!/bin/bash
cd ${0%/*}
. ../common.sh
# 段階 15：パイプと文の実行の高速化。片側だけが外部コマンドのパイプは OS のパイプでつなぐ
for c in case1
do
  run $c $c.sh < /dev/null
done

#!/bin/bash
# ConMa 版の構文解析（consh --conma-parser）が、parse1 と parse2 の各スクリプトについて、
# Python の consh_parse --pos と同じ構文木と同じエラーを出すかを調べる。エラーの前置きは consh_parse: と consh: の違いだけ
cd ${0%/*}
. ../common.sh
for f in ../parse1/case*.sh ../parse2/case*.sh
do
  d=${f%/*}; b=${f##*/}
  (cd $d && python3 "$consh/bin/consh_parse" --pos $b 2>&1; echo "exit: $?") | sed 's/^consh_parse: /consh: /' > /tmp/consh.parse3.expected
  (cd $d && "$consh/bin/consh" --conma-parser --dump $b 2>&1; echo "exit: $?") > /tmp/consh.parse3.actual
  if ! diff -q /tmp/consh.parse3.expected /tmp/consh.parse3.actual > /dev/null
  then
    echo "${d#../}/$b:"
    diff /tmp/consh.parse3.expected /tmp/consh.parse3.actual
  fi
done

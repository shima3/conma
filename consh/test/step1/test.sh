#!/bin/bash
cd ${0%/*}
. ../common.sh
# consh_parse --pos の構文木を ConMa 版で読み、書き戻した結果が元と同じになるか。
# parse1、parse2、expand1 のスクリプトのうち、構文エラーのないものすべてで確かめる。違えば diff を出す
for f in ../parse1/case*.sh ../parse2/case*.sh ../expand1/case*.sh
do
  python3 "$consh/bin/consh_parse" --pos $f > /tmp/consh.ref 2> /dev/null || continue
  "$consh/bin/consh" --dump $f > /tmp/consh.dump 2>&1
  if ! diff -q /tmp/consh.ref /tmp/consh.dump > /dev/null; then
    echo "== $f"
    diff /tmp/consh.ref /tmp/consh.dump
  fi
done
# case2 から case5: 壊れた構文木はエラーになる（consh_main.se に構文木のファイルを直接渡す）
for c in case2 case3 case4 case5
do
  PYTHONUNBUFFERED=1 python3 "$root/test/vproc_run.py" --main /tmp/consh.module.se dump $c.ast $c.ast > /tmp/consh.$c.log 2>&1
  echo "exit: $?" >> /tmp/consh.$c.log
  diff $c.log /tmp/consh.$c.log
done

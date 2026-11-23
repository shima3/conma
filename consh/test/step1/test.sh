#!/bin/bash
cd ${0%/*}
. ../common.sh
# bin/consh と同じモジュール（CONSH_CONMA_DATA なら ConMa 版のデータの関数を使うもの）
# （CONSH_MODULE_DIR か bin/cache-dir の場所）
if [ -n "$CONSH_MODULE_DIR" ]; then dir="$CONSH_MODULE_DIR"; else dir="$("$root/bin/cache-dir")" || exit 1; fi
module="$dir/consh.module.se"
[ -n "$CONSH_CONMA_DATA" ] && module="$dir/consh.data.module.se"
# 下の dump-sexp と case2 から case5 は、このモジュールを直に読む。CONSH_IMPL が rust（モジュールを作らない）か reference
# （consh.reference.module.se を作る）なら、consh はこれを作らないので、ConMa 版の最適化版の consh を 1 度動かして、ここで作る
# （ソースより古ければ作り直す）。作らないと、新しいクローンや worktree で、ConMa 版を流す前に CONSH_IMPL=rust だけで流したとき、
# モジュールがないエラーで違いになる（2026-11-23、shima3/conma#51 のレビュー）
if [ "$CONSH_IMPL" != conma ]; then
  CONSH_IMPL=conma "$consh/bin/consh" --dump ../parse1/case1.sh > /dev/null 2>&1 || echo "step1: could not make $module"
fi
# consh_parse --pos の構文木を ConMa 版で読み、書き戻した結果が元と同じになるか。
# parse1、parse2、expand1 のスクリプトのうち、構文エラーのないものすべてで確かめる。違えば diff を出す
for f in ../parse1/case*.sh ../parse2/case*.sh ../expand1/case*.sh
do
  python3 "$consh/bin/consh_parse" --pos $f > $tmp/ref 2> /dev/null || continue
  "$consh/bin/consh" --dump $f > $tmp/dump 2>&1
  if ! diff -q $tmp/ref $tmp/dump > /dev/null; then
    echo "== $f"
    diff $tmp/ref $tmp/dump
  fi
done
# ConMa で1文字ずつ読む Sexp_read（consh_main.se の dump-sexp）でも、同じ構文木になるか。parse1 で確かめる
for f in ../parse1/case*.sh
do
  python3 "$consh/bin/consh_parse" --pos $f > $tmp/ref 2> /dev/null || continue
  PYTHONUNBUFFERED=1 "$root/bin/vm" --main "$module" dump-sexp $tmp/ref $f > $tmp/dump 2>&1
  if ! diff -q $tmp/ref $tmp/dump > /dev/null; then
    echo "== $f (dump-sexp)"
    diff $tmp/ref $tmp/dump
  fi
done
# case2 から case5: 壊れた構文木はエラーになる（consh_main.se に構文木のファイルを直接渡す）。
# __Sexp_read__（dump）と Sexp_read（dump-sexp）で、同じメッセージになる
for mode in dump dump-sexp
do
  for c in case2 case3 case4 case5
  do
    PYTHONUNBUFFERED=1 "$root/bin/vm" --main "$module" $mode $c.ast $c.ast > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $c.log $tmp/$c.log
  done
done

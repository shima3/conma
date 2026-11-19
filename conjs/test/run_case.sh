#!/bin/bash
# ConJS のテストの 1 つのステップ（docs/dev/PLATFORM-PLAN-ja.md の 4.2 節）：そのディレクトリの case*.con.js を変換して動かし、
# 出力と終了ステータスを case*.log と比べる。VM は CONMA_VM（なし：Rust 版、wasm、python）。VM ごとに違うログは
# case*.$CONMA_VM.log に置く。CONMA_HOST（wasm 版の VM のホストの関数）は、case*.host.js があれば、それを使う。
# 標準エラーは、標準出力の後に続ける（Python 版の VM は、ファイルへの標準出力をためてから出すので、混ぜると順が VM で違う）
#   run_case.sh ステップのディレクトリ
cd "$1" || exit 1
conjs="$(cd ../..; pwd)/conjs"
out=${TMPDIR:-/tmp}/conjs-test-$$.log
err=${TMPDIR:-/tmp}/conjs-test-$$.err
for f in case*.con.js
do
  c=${f%.con.js}
  expected=$c.log
  [ -n "$CONMA_VM" ] && [ -f $c.$CONMA_VM.log ] && expected=$c.$CONMA_VM.log
  if [ -f $c.host.js ]; then
    CONMA_HOST=$PWD/$c.host.js "$conjs" $f > $out 2> $err < /dev/null
  else
    "$conjs" $f > $out 2> $err < /dev/null
  fi
  status=$?
  cat $err >> $out
  echo "exit: $status" >> $out
  diff $expected $out || echo "^ $1/$c (${CONMA_VM:-rust})"
done
rm -f $out $err

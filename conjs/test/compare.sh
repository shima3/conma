#!/bin/bash
# ConJS のプログラムのうち、spawn、join、host と待つ呼び出しのライブラリ（sleep、setTimeout、Date.now、fs、fetch）を
# 使わないものは、JavaScript のプログラムでもある。それを Node.js（node）と ConJS（../conjs。wasm 版の VM。段階 P10 から）で動かし、
# 標準出力が同じか比べる。違えば diff を出す（docs/dev/PLATFORM-PLAN-ja.md の 4.1 節の「ConJS を広げる道」の確かめ方）。
#   conjs/test/compare.sh [file.con.js ...]
# ファイルを書かなければ、各ステップの node.txt に並べた場合（2026-11-20 から。前は step10 のすべて）。変換するときの
# エラーの場合と、JavaScript との違いの場合は、node.txt に入れない。標準エラーと終了ステータスは比べない（例外の出し方が違う）
# ファイルを書かなければ、conjs/test/node-pairs.txt の組も比べる（段階 P8）：ConJS だけの API を使うプログラムと、同じことを
# Node.js で書いたプログラムの組。1 行に「ディレクトリ ConJS のファイル Node.js のファイル 引数...」（conjs/test から
# の相対パス。2 つのファイルは、そのディレクトリから見たパス）。そのディレクトリで、同じ引数で動かして比べる
here="$(cd "$(dirname "$0")"; pwd)"
pairs=
if [ $# -eq 0 ]; then
  pairs=yes
  for list in "$here"/step*/node.txt
  do
    while read -r c; do [ -n "$c" ] && set -- "$@" "${list%/node.txt}/$c"; done < "$list"
  done
fi
tmp="$(mktemp -d "${TMPDIR:-/tmp}/conjs-compare.XXXXXX")" || exit 1
trap 'rm -rf "$tmp"' EXIT
status=0
for f in "$@"
do
  (cd "$(dirname "$f")" && node "$(basename "$f")") > "$tmp/a.out" 2> /dev/null < /dev/null
  (cd "$(dirname "$f")" && "$here/../conjs" "$(basename "$f")") > "$tmp/b.out" 2> /dev/null < /dev/null
  if ! diff "$tmp/a.out" "$tmp/b.out" > "$tmp/diff"; then
    echo "== $f (< Node.js, > ConJS)"
    cat "$tmp/diff"
    status=1
  fi
done
if [ -n "$pairs" ] && [ -f "$here/node-pairs.txt" ]; then
  while read -r dir cfile nfile args; do
    case "$dir" in ''|'#'*) continue ;; esac
    (cd "$here/$dir" && node "$nfile" $args) > "$tmp/a.out" 2> /dev/null < /dev/null
    (cd "$here/$dir" && "$here/../conjs" "$cfile" $args) > "$tmp/b.out" 2> /dev/null < /dev/null
    if ! diff "$tmp/a.out" "$tmp/b.out" > "$tmp/diff"; then
      echo "== $dir: $nfile and $cfile $args (< Node.js, > ConJS)"
      cat "$tmp/diff"
      status=1
    fi
  done < "$here/node-pairs.txt"
fi
# ファイルを書かなければ、各ステップの case*.node-check.js も node で動かす（段階 P6b。サーバの見本は、ConJS の側が wasm 版
# の VM でしか動かないので、Node.js の出力を、そのステップの wasm 版のログと比べる）
if [ -n "$pairs" ]; then
  for check in "$here"/step*/case*.node-check.js
  do
    [ -f "$check" ] || continue
    node "$check" < /dev/null || status=1
  done
fi
exit $status

#!/bin/bash
# ConJS のプログラムのうち、spawn、join、host と待つ呼び出しのライブラリ（sleep、setTimeout、Date.now、fs、fetch）を
# 使わないものは、JavaScript のプログラムでもある。それを Node.js（node）と ConJS（../conjs。Rust 版の VM）で動かし、
# 標準出力が同じか比べる。違えば diff を出す（docs/dev/PLATFORM-PLAN-ja.md の 4.1 節の「ConJS を広げる道」の確かめ方）。
#   conjs/test/compare.sh [file.con.js ...]
# ファイルを書かなければ、各ステップの node.txt に並べた場合（2026-11-20 から。前は step10 のすべて）。変換するときの
# エラーの場合と、JavaScript との違いの場合は、node.txt に入れない。標準エラーと終了ステータスは比べない（例外の出し方が違う）
here="$(cd "$(dirname "$0")"; pwd)"
if [ $# -eq 0 ]; then
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
exit $status

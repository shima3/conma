#!/bin/bash
# SJS のプログラムを、段階 B（ConMa に変換して動かす。../sjs）と、段階 A（ConSh の (( )) でホストの JavaScript。
# wasm 版の VM を Node.js で動かす）で動かし、出力が同じか比べる。違えば diff を出す。
#   consh/sjs/test/compare.sh file.sjs ...
# 段階 A では、プログラムを (( (() => { ... })() )) の関数の本体として動かす。エラーの行の前置き
# （段階 A の consh: ファイル:行:桁: と、段階 B の line 行: ）は除いて比べる。終了ステータスは比べない（段階 A のスクリプトは続く）
here="$(cd "$(dirname "$0")"; pwd)"
consh="${here%/*/*}"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/sjs-compare.XXXXXX")" || exit 1
trap 'rm -rf "$tmp"' EXIT
status=0
for f in "$@"
do
  { printf '(( (() => {\n'; cat "$f"; printf '\n})() ))\n'; } > "$tmp/a.sh"
  CONSH_ENABLE_JS=sjs CONMA_VM=wasm CONSH_IMPL=conma "$consh/bin/consh" "$tmp/a.sh" < /dev/null 2>&1 \
    | sed -e 's/^consh: [^ ]*:[0-9]*:[0-9]*: sjs: /sjs: /' > "$tmp/a.out"
  "$consh/sjs/sjs" "$f" 2>&1 | sed -e 's/^sjs: line [0-9]*: /sjs: /' > "$tmp/b.out"
  if ! diff "$tmp/a.out" "$tmp/b.out" > "$tmp/diff"; then
    echo "== $f (< stage A, > stage B)"
    cat "$tmp/diff"
    status=1
  fi
done
exit $status

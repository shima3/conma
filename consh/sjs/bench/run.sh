#!/bin/bash
# consh/sjs/bench/run.sh NAME N：SJS の段階 B7 の測定（consh/SAFEJS-PLAN-ja.md の 6）。
# NAME.sjs（N_ARG を N にしたもの）と、bench/NAME（CPython と、手で書いた ConMa）の同じ計算について、
# 大きさ（SJS、変換した ConMa のソース、モジュール）、変換の時間、実行の時間（user + sys 秒、3 回の最良）を出す。
# 実行は、Node.js、CPython、Rust 版の VM（手で書いた ConMa と、変換した SJS）。出力が同じことも確かめる。
# wasm 版の VM（consh/js/build.sh で作り、Node.js で動かす）があれば、その上で、変換した SJS（数の関数は WebAssembly。
# consh/SJS-WASM-PLAN-ja.md）、CONMA_NO_WASM=1 で ConMa 版だけ、手で書いた ConMa も測る
sjsbench="$(cd "$(dirname "$0")"; pwd)"
sjs="${sjsbench%/*}"
consh="${sjs%/*}"
root="${consh%/*}"
conma="$root/vm/target/release/conma"
name=$1 n=$2
# SJS_INCLUDE で、変換器と実行時の支えのディレクトリを替えられる（前の版と比べるとき）
inc="${SJS_INCLUDE:-$consh/include}"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/sjs-bench.XXXXXX")" || exit 1
trap 'rm -rf "$tmp"' EXIT
sed "s/N_ARG/$n/" "$sjsbench/$name.sjs" > "$tmp/$name.sjs"
best() {  # command...：3 回の最良の user + sys 秒
  local b= t
  for i in 1 2 3; do
    t=$( { TIMEFORMAT='%U %S'; time "$@" > /dev/null 2>&1; } 2>&1 | awk '{print $1 + $2}')
    if [ -z "$b" ] || awk "BEGIN{exit !($t < $b)}"; then b=$t; fi
  done
  echo "$b"
}
size() { wc -c < "$1" | tr -d ' '; }
lines() { grep -cv '^\s*\(//.*\)\?$' "$1"; }
# 変換（変換器のモジュールを作る時間を含まないように、変換器を先に作る）
"$conma" build --include "$inc" "$sjs/sjsc.se" > "$tmp/sjsc.module.se" || exit 1
"$conma" --main "$tmp/sjsc.module.se" "$tmp/$name.sjs" > "$tmp/$name.se" || exit 1
translate=$(best "$conma" --main "$tmp/sjsc.module.se" "$tmp/$name.sjs")
"$conma" build --include "$inc" --include "$root/include" "$tmp/$name.se" > "$tmp/$name.module.se" || exit 1
"$root/bench/module.sh" "$root/bench/$name/$name.se" "$tmp/hand.module.se" > /dev/null || exit 1
# 出力
node "$tmp/$name.sjs" > "$tmp/node.out"
"$conma" --main "$tmp/$name.module.se" > "$tmp/sjs.out" 2>&1
python3 "$root/bench/$name/$name.py" "$n" > "$tmp/py.out"
"$conma" --main "$tmp/hand.module.se" "$n" > "$tmp/hand.out" 2>&1
cmp -s "$tmp/node.out" "$tmp/sjs.out" || { echo "SJS prints otherwise than Node.js:"; diff "$tmp/node.out" "$tmp/sjs.out"; }
echo "== $name $n"
printf 'size: SJS %s lines %s bytes; translated ConMa %s bytes (%s lines); module %s bytes (with sjs_rt.se); hand-written ConMa module %s bytes\n' \
  "$(lines "$sjsbench/$name.sjs")" "$(size "$sjsbench/$name.sjs")" "$(size "$tmp/$name.se")" "$(wc -l < "$tmp/$name.se" | tr -d ' ')" \
  "$(size "$tmp/$name.module.se")" "$(size "$tmp/hand.module.se")"
printf 'translate (sjsc on the Rust VM): %s s\n' "$translate"
printf '%-24s %s s\n' "Node.js" "$(best node "$tmp/$name.sjs")"
printf '%-24s %s s\n' "CPython" "$(best python3 "$root/bench/$name/$name.py" "$n")"
printf '%-24s %s s\n' "Rust VM, hand ConMa" "$(best "$conma" --main "$tmp/hand.module.se" "$n")"
printf '%-24s %s s\n' "Rust VM, SJS" "$(best "$conma" --main "$tmp/$name.module.se")"
vmw="$consh/js/vm-wasm.sh"
if [ -f "$consh/js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm" ]; then
  "$vmw" --main "$tmp/$name.module.se" > "$tmp/wasm.out" 2>&1
  cmp -s "$tmp/node.out" "$tmp/wasm.out" || { echo "SJS on the wasm VM prints otherwise than Node.js:"; diff "$tmp/node.out" "$tmp/wasm.out"; }
  nfun=$(grep -o 'Sjs_wasm_fn Wsjs "[^"]*"' "$tmp/$name.se" | wc -l | tr -d ' ')
  # WAT の文字列の長さ（\" などのエスケープを 1 文字と数える。REVIEW-ja.md の 2026-11-14 (2) の 8）
  wat=$(perl -ne 'if (/Sjs_wasm_module "((?:[^"\\]|\\.)*)"/) { ($s = $1) =~ s/\\(.)/$1/g; print length($s) }' "$tmp/$name.se")
  wat=${wat:-0}
  printf 'numeric functions: %s (WAT %s bytes)\n' "$nfun" "$wat"
  printf '%-24s %s s\n' "wasm VM, SJS" "$(best "$vmw" --main "$tmp/$name.module.se")"
  printf '%-24s %s s\n' "wasm VM, SJS, no wasm" "$(best env CONMA_NO_WASM=1 "$vmw" --main "$tmp/$name.module.se")"
  printf '%-24s %s s\n' "wasm VM, hand ConMa" "$(best "$vmw" --main "$tmp/hand.module.se" "$n")"
fi
# 起動の時間（N が SMALL のとき）と、VM のステップの数（CONMA_PROFILE。N が SMALL と SMALL + 10 のときの差を 10 で割る）
small=${SMALL:-1}
mk() {  # n file：N_ARG を n にした SJS のモジュール
  sed "s/N_ARG/$1/" "$sjsbench/$name.sjs" > "$tmp/m.sjs"
  "$conma" --main "$tmp/sjsc.module.se" "$tmp/m.sjs" > "$tmp/m.se" &&
  "$conma" build --include "$inc" --include "$root/include" "$tmp/m.se" > "$2"
}
steps() { CONMA_PROFILE="$tmp/prof" "$conma" --main "$@" > /dev/null 2>&1; python3 "$root/test/vm_profile.py" "$tmp/prof" | sed -n 's/^steps: //p' | tr -d ,; }
mk "$small" "$tmp/s1.module.se"
mk "$((small + 10))" "$tmp/s2.module.se"
printf '%-24s %s s (N = %s)\n' "Rust VM, SJS, start" "$(best "$conma" --main "$tmp/s1.module.se")" "$small"
printf '%-24s %s s (N = %s)\n' "Rust VM, hand, start" "$(best "$conma" --main "$tmp/hand.module.se" "$small")" "$small"
s1=$(steps "$tmp/s1.module.se"); s2=$(steps "$tmp/s2.module.se")
h1=$(steps "$tmp/hand.module.se" "$small"); h2=$(steps "$tmp/hand.module.se" "$((small + 10))")
echo "steps for N = $small: SJS $s1, hand $h1; steps for each N more: SJS $(( (s2 - s1) / 10 )), hand $(( (h2 - h1) / 10 ))"

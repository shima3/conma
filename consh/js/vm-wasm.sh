#!/bin/bash
# ConMa の VM の wasm32 版（consh/js/wasm）を Node.js（vm-node.js）か、macOS の JavaScriptCore のシェル
# （jsc、vm-jsc.js）で動かす。引数は vm/target/release/conma と同じ（[--trace] [--main] <module_file> <func_name | arg ...>）。
# test/vm.sh が CONMA_VM=wasm のときに使う。wasm は consh/js/build.sh で作っておく。
# 環境変数 CONMA_WASM_HOST が node なら Node.js、jsc なら jsc。なければ、node が動けば Node.js、動かなければ jsc
# 標準入力は、VM がそれを待つときに、読めた分ずつ渡す。
dir="$(cd "${0%/*}"; pwd)"
jsc="${JSC:-/System/Library/Frameworks/JavaScriptCore.framework/Versions/Current/Helpers/jsc}"
wasm="$dir/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm"
if [ ! -f "$wasm" ]; then
  echo "vm-wasm.sh: $wasm is not built (consh/js/build.sh)" >&2
  exit 2
fi
host="$CONMA_WASM_HOST"
if [ -z "$host" ]; then
  if node -e 0 > /dev/null 2>&1; then host=node; else host=jsc; fi
fi
case "$host" in
  node) exec node "$dir/vm-node.js" "$@" ;;
  jsc) ;;
  *) echo "vm-wasm.sh: CONMA_WASM_HOST must be node or jsc, not \"$host\"" >&2; exit 2 ;;
esac
# 以下は jsc。jsc はバイト列を読み書きできず、終了コードも返せないので、16 進数の行にしてつなぐ（vm-jsc.js）
# 標準入力を "d <16進数>" の行（読めた分ずつ）と、終わりの "e" の行にして jsc に渡す。
# jsc が終わったら、まだ読んでいれば止める（端末などの入力を、後のコマンドから取らないように）
exec 3< <(python3 -c '
import os, sys
while True:
    try:
        b = os.read(0, 65536)
    except OSError:
        b = b""
    try:
        sys.stdout.write("d " + b.hex() + "\n" if b else "e\n")
        sys.stdout.flush()
    except BrokenPipeError:
        break
    if not b:
        break
')
feeder=$!
# 環境変数（NUL で区切る）。VM は今のディレクトリで始まる
envfile="$(mktemp "${TMPDIR:-/tmp}/conma-wasm-env.XXXXXX")"
env -0 > "$envfile"
# 今のディレクトリの下（深さ 3 まで）のディレクトリとファイルの名前。ファイルの中身は、VM が開いたときに読む。
# jsc はディレクトリを調べられないので、ここで渡す
dirs="$(mktemp "${TMPDIR:-/tmp}/conma-wasm-dirs.XXXXXX")"
files="$(mktemp "${TMPDIR:-/tmp}/conma-wasm-files.XXXXXX")"
trap 'rm -f "$envfile" "$dirs" "$files"' EXIT
find . -maxdepth 3 -type d -print0 2>/dev/null > "$dirs"
# 一時ファイルの置き場所の下のディレクトリも（テストが作ったものなど。中のファイルは、開いたときに読む）
for t in /tmp "${TMPDIR%/}"; do
  [ -d "$t" ] && find "$t/" -maxdepth 2 -type d -print0 2>/dev/null >> "$dirs"
done
find . -maxdepth 3 ! -type d -print0 2>/dev/null > "$files"
# jsc が出す、見つからないファイルのメッセージ（ファイルを探したときのもの）は消す
"$jsc" "$dir/vm-jsc.js" -- "$dir" "$(pwd -P)" "$envfile" "$dirs" "$files" "$@" <&3 3<&- 2> >(grep -v '^Could not open file: ' >&2) | python3 -c '
import sys
code = 2
for line in sys.stdin.buffer:
    kind, _, data = line.rstrip(b"\n").partition(b" ")
    if kind == b"x":
        code = int(data)
        break
    out = sys.stdout.buffer if kind == b"1" else sys.stderr.buffer
    out.write(bytes.fromhex(data.decode()))
    out.flush()
sys.exit(code)
'
code=$?
exec 3<&-
kill $feeder 2>/dev/null
rm -f "$envfile" "$dirs" "$files"
exit $code

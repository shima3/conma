#!/bin/bash
# ConMa の VM の wasm32 版（consh/js/wasm）を JavaScriptCore のシェル（macOS の jsc）で動かす。
# 引数は vm/target/release/conma と同じ（[--trace] [--main] <module_file> <func_name | arg ...>）。
# test/vm.sh が CONMA_VM=wasm のときに使う。wasm は consh/js/build.sh で作っておく。
# 標準入力は、VM がそれを待つときに、読めた分ずつ渡す。
dir="$(cd "${0%/*}"; pwd)"
jsc="${JSC:-/System/Library/Frameworks/JavaScriptCore.framework/Versions/Current/Helpers/jsc}"
wasm="$dir/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm"
if [ ! -f "$wasm" ]; then
  echo "vm-wasm.sh: $wasm is not built (consh/js/build.sh)" >&2
  exit 2
fi
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
"$jsc" "$dir/vm-jsc.js" -- "$dir" "$@" <&3 3<&- | python3 -c '
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
exit $code

#!/bin/bash
# __Feature_available__（docs/PRIMITIVES.md の 15）と、プラットフォームの機能 "js" の印（docs/dev/PLATFORM-PLAN-ja.md の段階 P1）。
# フロントエンド（bin/resolver と、ConMa 版の include/resolver.se）が、__JS_call__ と __JS_value__ を使うモジュールに
# (Platform "js") を付け、bin/to-core は (Uses ...) を除いても (Platform ...) を残す（プラットフォームの機能は言語の層とは別で、
# Python の VM もそのプリミティブを持つ）。
# ログは VM ごとに違う：caseN.wasm.log は wasm 版の VM（Node.js。WebAssembly と JavaScript がある）、
# caseN.wasm_nowasm.log は wasm 版の VM で CONMA_NO_WASM=1（WebAssembly を使わない）、caseN.log はそれ以外（Python、Rust、変換版）。
# ないものは caseN.log を使う
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
head -1 $tmp/test.module.se | grep -q '(Uses "control") (Platform "js")$' || echo "bin/resolver did not write (Uses \"control\") (Platform \"js\")"
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast13/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/test.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
"$root/bin/to-core" $tmp/test.module.se > $tmp/core.module.se || exit 1
head -1 $tmp/core.module.se | grep -q '(Uses' && echo "bin/to-core left (Uses ...)"
head -1 $tmp/core.module.se | grep -q '(Platform "js")' || echo "bin/to-core removed (Platform \"js\")"
vm="${CONMA_VM:-python}"
[ "$vm" = transpiled ] && vm=rust
for c in case1 case2 case3
do
  log=$c.$vm.log
  [ -f $log ] || log=$c.log
  PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log
done
if [ "$vm" = wasm ]; then
  for c in case1
  do
    CONMA_NO_WASM=1 PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $c.wasm_nowasm.log $tmp/$c.log
  done
fi

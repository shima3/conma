#!/bin/bash
# 高速版 ConMa の機能 "wasm"（consh/SJS-WASM-PLAN-ja.md の段階 1）：__Wasm_compile__ と __Wasm_call__。
# ログは VM ごとに違う：caseN.wasm.log は wasm 版の VM（Node.js。ホストの WebAssembly で動かす）、caseN.python.log は
# Python の VM（代わりの定義 include/ref/wasm.se で onError）、caseN.log はそれ以外（ネイティブの VM は WAT を組み立ててから
# onError。組み立ての誤りはそのメッセージ）。ないものは caseN.log を使う。
# wasm 版の VM で CONMA_NO_WASM=1 なら、ネイティブの VM と同じ（WebAssembly を使わない）。
# case7 は、燃料を使い切るたびに時間の区切りが終わり、SIGINT が届くこと（段階 3）。case8 は、モジュールのメモリと
# __Wasm_call_memory__（consh/SJS-WASM-ARRAY-PLAN-ja.md の段階 A1）。
# フロントエンド（bin/resolver と conma build）が (Uses "boolean" "control" "wasm") を付け、bin/concise が代わりの定義を入れる
. "$(dirname "$0")/../common.sh"
CONMA_FAST=1 resolve test.se
head -1 $tmp/test.module.se | grep -q '(Uses "boolean" "control" "wasm")$' || echo "bin/resolver did not write (Uses \"boolean\" \"control\" \"wasm\")"
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast7/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/test.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
"$root/bin/concise" $tmp/test.module.se > $tmp/concise.module.se || exit 1
grep -q '(Uses' $tmp/concise.module.se && echo "bin/concise left (Uses ...)"
grep -q '(G [0-9]* "__Wasm_call__"$' $tmp/concise.module.se || echo "bin/concise did not put the definition of __Wasm_call__"
vm="${CONMA_VM:-python}"
[ "$vm" = transpiled ] && vm=rust
for c in case1 case2 case3 case4 case5 case6 case7 case8
do
  log=$c.$vm.log
  [ -f $log ] || log=$c.log
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log
done
if [ "$vm" = wasm ]; then
  for c in case1 case5
  do
    CONMA_NO_WASM=1 PYTHONUNBUFFERED=1 ../vm.sh $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $c.log $tmp/$c.log
  done
fi

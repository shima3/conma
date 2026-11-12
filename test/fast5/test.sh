#!/bin/bash
# 高速版 ConMa の機能 "bitwise32"（docs/BITWISE-PLAN-ja.md の W2）：ビット演算の 7 つのプリミティブ。
# step41 と同じ式を、代わりの定義（include/fast/bitwise32.se）を入れずに、モジュールに (Uses "bitwise32") を付けて動かす。
# 高速版の VM（Rust、wasm、変換版）は、プリミティブで step41 と同じログ（Node.js の値）になる。
# 簡潔版の VM（Python）は、機能を知らないのでエラーにする（concise.log）
. "$(dirname "$0")/../common.sh"
resolve test.se
perl -pe 's/^(\(Module \(F [^)]*\))/$1 (Uses "bitwise32")/ if $. == 1' $tmp/test.module.se > $tmp/fast.module.se
for c in case1 case2 case3
do
  PYTHONUNBUFFERED=1 ../vm.sh $tmp/fast.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  case "${CONMA_VM:-python}" in
    rust|wasm|transpiled) diff ../step41/$c.log $tmp/$c.log ;;
    *) diff concise.log $tmp/$c.log ;;
  esac
done

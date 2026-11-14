#!/bin/bash
# 配列の計画（consh/SJS-WASM-ARRAY-PLAN-ja.md）の段階 A2：実行時の支え（sjs_rt.se の Sjs_wasm_pack、Sjs_wasm_unpack）が、配列を
# wasm のモジュールのメモリに写し、"ok" なら書き戻す。手で書いた WAT と、ConMa で書いた ConMa 版（"[ConMa]" を出す）で確かめる。
# case1.log は Rust 版の VM（ConMa 版で動く）、case1.wasm.log は wasm 版の VM（Node.js と consh/js/build.sh が要る。なければ飛ばす）
cd ${0%/*}
root=../../../..
conma=$root/vm/target/release/conma
out=${TMPDIR:-/tmp}/sjs-test-$$.log
mod=${TMPDIR:-/tmp}/sjs-test-$$.module.se
"$conma" run --include ../../../include case1.se > $out 2>&1
echo "exit: $?" >> $out
diff case1.log $out
if [ -f ../../../js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm ] && command -v node > /dev/null; then
  "$conma" build --include ../../../include case1.se > $mod
  ../../../js/vm-wasm.sh --main $mod > $out 2>&1
  echo "exit: $?" >> $out
  diff case1.wasm.log $out
fi
rm -f $out $mod

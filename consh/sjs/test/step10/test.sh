#!/bin/bash
# 数の関数の wasm 版の燃料と trap（consh/SJS-WASM-PLAN-ja.md の段階 3）：wasm 版の VM（CONMA_VM=wasm。Node.js と
# consh/js/build.sh が要る。なければ飛ばす）だけで流す。case1 のログは Node.js の結果、case2 は手で書いた
cd ${0%/*}
[ -f ../../../js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm ] && command -v node > /dev/null || exit 0
out=/tmp/sjs-test-$$.log
for f in case*.sjs
do
  c=${f%.sjs}
  CONMA_VM=wasm ../../sjs $f > $out 2>&1
  echo "exit: $?" >> $out
  diff $c.log $out
done
rm -f $out

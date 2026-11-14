#!/bin/bash
# 数の関数の wasm 版（consh/SJS-WASM-PLAN-ja.md の段階 2）：ログは Node.js で同じプログラムを動かした結果。
# Rust 版の VM（数の関数も ConMa 版で動く）と、wasm 版の VM（CONMA_VM=wasm。数の関数は wasm 版で動く。Node.js と
# consh/js/build.sh が要る。なければ飛ばす）の両方で比べる。--emit の出力で、数の関数になったものを確かめる
cd ${0%/*}
out=/tmp/sjs-test-$$.log
vms=(rust)
if [ -f ../../../js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm ] && command -v node > /dev/null; then vms+=(wasm); fi
for f in case*.sjs
do
  c=${f%.sjs}
  for vm in "${vms[@]}"
  do
    if [ $vm = wasm ]; then SJS_WASM_STRICT=1 CONMA_VM=wasm ../../sjs $f > $out 2>&1; else ../../sjs $f > $out 2>&1; fi
    echo "exit: $?" >> $out
    diff $c.log $out
  done
  # 数の関数になったもの（Sjs_wasm_fn で包んだ関数の名前）
  ../../sjs --emit $f | grep -o 'Sjs_wasm_fn Wsjs "[^"]*"' | sed 's/.*"\(.*\)"/\1/' | tr '\n' ' ' > $out
  echo >> $out
  diff $c.wasm.txt $out
done
rm -f $out

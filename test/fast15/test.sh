#!/bin/bash
# 待つホストの呼び出し（__Host_start__、__Host_start_tracked__、__Host_result__。docs/PRIMITIVES.md、
# docs/dev/PLATFORM-PLAN-ja.md の 3.4 節、段階 P3）。フロントエンドが、これらを使うモジュールに (Platform "host") を付ける。
# case1 は、どの VM にもあるホストの関数（機能 "test"）とエラー。case2 は、test.later の終わる順（仮の時計）。
# case3 は、待っている VProc を殺して、__Host_start_tracked__ で記録した Stream を閉じる場合。
# case4 は、wasm 版の VM のホストの JavaScript の関数（CONMA_HOST=host.js。Promise を返すものも）。
# ログは、どの VM でも同じ（case4 だけは、wasm 版の VM が case4.wasm.log、ほかの VM が case4.log）
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
head -1 $tmp/test.module.se | grep -q '(Platform "host")$' || echo "bin/resolver did not write (Platform \"host\")"
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast15/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/test.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
vm="${CONMA_VM:-python}"
[ "$vm" = transpiled ] && vm=rust
for c in case1 case2 case3 case4
do
  log=$c.$vm.log
  [ -f $log ] || log=$c.log
  CONMA_HOST="$(pwd)/host.js" PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log
done

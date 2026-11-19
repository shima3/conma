#!/bin/bash
# __Host_call__（docs/PRIMITIVES.md、docs/dev/PLATFORM-PLAN-ja.md の段階 P2）と、プラットフォームの機能 "host" の印。
# フロントエンド（bin/resolver と、ConMa 版の include/resolver.se）が、__Host_call__ を使うモジュールに (Platform "host") を付ける。
# case1 は、どの VM にもあるホストの関数（機能 "test"：test.add、test.echo、test.fail）とエラーで、どの VM でも同じログ。
# case2 は、wasm 版の VM のホストの JavaScript の関数（CONMA_HOST=host.js。値を JSON で渡し、-0、NaN、無限大、例外、
# 渡せない値）。ログは、wasm 版の VM が case2.wasm.log、ほかの VM が case2.log（その関数がない）
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
head -1 $tmp/test.module.se | grep -q '(Platform "host")$' || echo "bin/resolver did not write (Platform \"host\")"
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast14/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/test.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
vm="${CONMA_VM:-python}"
[ "$vm" = transpiled ] && vm=rust
for c in case1 case2
do
  log=$c.$vm.log
  [ -f $log ] || log=$c.log
  CONMA_HOST="$(pwd)/host.js" PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log
done

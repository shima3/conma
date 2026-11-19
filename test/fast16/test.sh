#!/bin/bash
# canvas のライブラリ（include/platform/canvas.se。docs/dev/PLATFORM-PLAN-ja.md の 3.4、段階 P3）。
# case1 と case1b は、canvas のない VM（CONMA_HOST なし）で、どの VM でも同じログ。
# case2 は、wasm 版の VM で、描いた命令を記録する canvas のまね（CONMA_HOST=host.js、vm/wasm/host/canvas.js を使う）。
# ログは、wasm 版の VM が case2.wasm.log、ほかの VM が case2.log（canvas がない）。
# 最後に、ライブラリと見本が使う canvas の関数の名前が、docs/PRIMITIVES.md の一覧にあることを確かめる
# （知らない名前は黙って onError や偽になり、書き間違いと区別できないため）
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
vm="${CONMA_VM:-python}"
[ "$vm" = transpiled ] && vm=rust
for c in case1 case1b case2
do
  log=$c.$vm.log
  [ -f $log ] || log=$c.log
  host=
  [ $c = case2 ] && host="$(pwd)/host.js"
  CONMA_HOST="$host" PYTHONUNBUFFERED=1 ../../bin/vm $tmp/test.module.se $c < /dev/null > $tmp/$c.log 2>&1
  echo "exit: $?" >> $tmp/$c.log
  diff $log $tmp/$c.log
done
for name in $(cat "$root/include/platform/canvas.se" "$root"/vm/wasm/web/canvas*.se 2>/dev/null | grep -o 'Canvas_call onError "[a-z_]*"' | sed 's/.*"\(.*\)"/canvas.\1/' | sort -u) canvas.size canvas.frame
do
  grep -q "\`$name\`" "$root/docs/PRIMITIVES.md" || echo "$name is not in docs/PRIMITIVES.md"
done

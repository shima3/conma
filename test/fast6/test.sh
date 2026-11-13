#!/bin/bash
# 高速版 ConMa の機能 "boolean"：__TRUE__ と __FALSE__ の組み込み。高速版（include/fast を先に探す）で作ると、
# include/fast/pure.se には定義がないので、フロントエンド（bin/resolver と conma build）がモジュールに (Uses "boolean") を付ける。
# 高速版の VM（Rust、wasm、変換版）はプリミティブで、簡潔版の VM（Python）は test/vm.sh が bin/concise で簡潔版に
# 直したモジュール（include/ref/pure.se の定義を入れたもの）で動かし、どれも同じログになる。
# bin/concise で直したモジュールを、それぞれの VM で動かしても同じ
. "$(dirname "$0")/../common.sh"
CONMA_FAST=1 resolve test.se
cp $tmp/test.module.se $tmp/fast.module.se
head -1 $tmp/fast.module.se | grep -q '(Uses "boolean" "control")$' || echo "bin/resolver did not write (Uses \"boolean\" \"control\")"
# ConMa 版のフロントエンド（conma build）も、同じモジュールを作る
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast6/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/fast.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
"$root/bin/concise" $tmp/fast.module.se > $tmp/concise.module.se || exit 1
grep -q '(Uses' $tmp/concise.module.se && echo "bin/concise left (Uses ...)"
grep -q '(G [0-9]* "__TRUE__"$' $tmp/concise.module.se || echo "bin/concise did not put the definition of __TRUE__"
for m in fast concise
do
  for c in case1 case2
  do
    PYTHONUNBUFFERED=1 ../vm.sh $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $c.log $tmp/$c.log
  done
done

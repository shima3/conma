#!/bin/bash
# 拡張 ConMa の機能 "boolean"：__TRUE__ と __FALSE__ の組み込み。拡張 ConMa（include/ext を先に探す）で作ると、
# include/ext/pure.se には定義がないので、フロントエンド（bin/resolver と conma build）がモジュールに (Uses "boolean") を付ける。
# 拡張 ConMa の VM（Rust、wasm、変換版）はプリミティブで、コア ConMa の VM（Python）は test/vm.sh が bin/to-core でコア ConMa に
# 直したモジュール（include/core/pure.se の定義を入れたもの）で動かし、どれも同じログになる。
# bin/to-core で直したモジュールを、それぞれの VM で動かしても同じ
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
cp $tmp/test.module.se $tmp/ext.module.se
head -1 $tmp/ext.module.se | grep -q '(Uses "boolean" "control")$' || echo "bin/resolver did not write (Uses \"boolean\" \"control\")"
# ConMa 版のフロントエンド（conma build）も、同じモジュールを作る
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast6/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/ext.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
"$root/bin/to-core" $tmp/ext.module.se > $tmp/core.module.se || exit 1
grep -q '(Uses' $tmp/core.module.se && echo "bin/to-core left (Uses ...)"
grep -q '(G [0-9]* "__TRUE__"$' $tmp/core.module.se || echo "bin/to-core did not put the definition of __TRUE__"
for m in ext core
do
  for c in case1 case2
  do
    PYTHONUNBUFFERED=1 ../vm.sh $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $c.log $tmp/$c.log
  done
done

#!/bin/bash
# 拡張 ConMa の機能 "chan"（docs/dev/CHAN-PLAN-ja.md の C2、C3）：チャネルの 8 つのプリミティブ。
# step45 と同じプログラムを、代わりの定義（include/core/chan.se）を入れずに動かす。フロントエンド（bin/resolver と、ConMa 版の
# include/resolver.se）が、モジュールに (Uses "boolean" "chan" "control") を付ける。
# 拡張 ConMa の VM（Rust、wasm、変換版）はプリミティブで、コア ConMa の VM（Python）は bin/vm が bin/to-core でコア ConMa に
# 直したモジュール（代わりの定義を入れたもの）で、どれも step45 と同じログになる。
# bin/to-core で直したモジュールを、それぞれの VM で動かしても同じ（代わりの定義とプリミティブが同じ動きをする）。
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
cp $tmp/test.module.se $tmp/ext.module.se
head -1 $tmp/ext.module.se | grep -q '(Uses "boolean" "chan" "control")$' || echo "bin/resolver did not write (Uses \"boolean\" \"chan\" \"control\")"
# ConMa 版のフロントエンド（conma build）も、同じモジュールを作る
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast11/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/ext.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
"$root/bin/to-core" $tmp/ext.module.se > $tmp/core.module.se || exit 1
grep -q '(Uses' $tmp/core.module.se && echo "bin/to-core left (Uses ...)"
grep -q '(G [0-9]* "__Chan_take__"$' $tmp/core.module.se || echo "bin/to-core did not put the definition of __Chan_take__"
for m in ext core
do
  for c in case1 case2 case3 case4 case5 case6
  do
    PYTHONUNBUFFERED=1 ../../bin/vm $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff ../step45/$c.log $tmp/$c.log
  done
done

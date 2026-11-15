#!/bin/bash
# 拡張 ConMa の機能 "bitwise32"（docs/BITWISE-PLAN-ja.md の W2、W3）：ビット演算の 7 つのプリミティブ。
# step41 と同じ式を、代わりの定義（include/core/bitwise32.se）を入れずに動かす。フロントエンド（bin/resolver と、ConMa 版の
# include/resolver.se。W4）が、モジュールに (Uses "bitwise32") を付ける。
# 拡張 ConMa の VM（Rust、wasm、変換版）はプリミティブで、コア ConMa の VM（Python）は bin/vm が bin/to-core でコア ConMa に
# 直したモジュール（代わりの定義を入れたもの）で、どれも step41 と同じログ（Node.js の値）になる。
# bin/to-core で直したモジュールを、それぞれの VM で動かしても同じ（代わりの定義とプリミティブが同じ値になる）。
# own.se：自分で定義した名前は、直しても自分の定義のまま
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se own.se
cp $tmp/test.module.se $tmp/ext.module.se
head -1 $tmp/ext.module.se | grep -q '(Uses "bitwise32" "boolean" "control")$' || echo "bin/resolver did not write (Uses \"bitwise32\" \"boolean\" \"control\")"
# ConMa 版のフロントエンド（conma build）も、同じモジュールを作る
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast5/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/ext.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
"$root/bin/to-core" $tmp/ext.module.se > $tmp/core.module.se || exit 1
grep -q '(Uses' $tmp/core.module.se && echo "bin/to-core left (Uses ...)"
grep -q '(G [0-9]* "__Number_xor32__"$' $tmp/core.module.se || echo "bin/to-core did not put the definition of __Number_xor32__"
for m in ext core
do
  for c in case1 case2 case3
  do
    PYTHONUNBUFFERED=1 ../../bin/vm $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff ../step41/$c.log $tmp/$c.log
  done
done
# 自分で定義した名前は、bin/to-core が代わりの定義で替えない
PYTHONUNBUFFERED=1 ../../bin/vm --main $tmp/own.module.se < /dev/null > $tmp/own.log 2>&1
echo "exit: $?" >> $tmp/own.log
diff own.log $tmp/own.log

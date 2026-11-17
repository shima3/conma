#!/bin/bash
# 拡張 ConMa の機能 "array"（docs/dev/ARRAY-PLAN-ja.md の A2、A3）：配列の 6 つのプリミティブ。
# step47 と同じプログラムを、代わりの定義（include/core/array.se）を入れずに動かす。フロントエンド（bin/resolver と、ConMa 版の
# include/resolver.se）が、モジュールに (Uses "array" "boolean" "control") を付ける。
# 拡張 ConMa の VM（Rust、wasm、変換版）はプリミティブで、コア ConMa の VM（Python）は bin/vm が bin/to-core でコア ConMa に
# 直したモジュール（代わりの定義を入れたもの）で、どれも step47 と同じログになる。
# bin/to-core で直したモジュールを、それぞれの VM で動かしても同じ（代わりの定義とプリミティブが同じ動きをする）。
# 配列の __type_name__ は VM によって違う（プリミティブは "Array"、代わりの定義は "Dict"）ので、case6 で、ext のモジュールの
# ものだけを確かめる（Python 版の VM は ext のモジュールも bin/to-core で直して動かすので、"Dict" になる）
. "$(dirname "$0")/../common.sh"
CONMA_EXT=1 resolve test.se
cp $tmp/test.module.se $tmp/ext.module.se
head -1 $tmp/ext.module.se | grep -q '(Uses "array" "boolean" "control")$' || echo "bin/resolver did not write (Uses \"array\" \"boolean\" \"control\")"
# ConMa 版のフロントエンド（conma build）も、同じモジュールを作る
conma="$root/vm/target/release/conma"
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast12/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/ext.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
"$root/bin/to-core" $tmp/ext.module.se > $tmp/core.module.se || exit 1
grep -q '(Uses' $tmp/core.module.se && echo "bin/to-core left (Uses ...)"
grep -q '(G [0-9]* "__Array_get__"$' $tmp/core.module.se || echo "bin/to-core did not put the definition of __Array_get__"
for m in ext core
do
  for c in case1 case2 case3 case4 case5
  do
    PYTHONUNBUFFERED=1 ../../bin/vm $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff ../step47/$c.log $tmp/$c.log
  done
done
# __type_name__ of an array
if [ "${CONMA_VM:-python}" = python ]; then expected=Dict; else expected=Array; fi
PYTHONUNBUFFERED=1 ../../bin/vm $tmp/ext.module.se case6 < /dev/null > $tmp/case6.log 2>&1
[ "$(cat $tmp/case6.log)" = "$expected" ] || { echo "case6: expected $expected, got:"; cat $tmp/case6.log; }
PYTHONUNBUFFERED=1 ../../bin/vm $tmp/core.module.se case6 < /dev/null > $tmp/case6.log 2>&1
[ "$(cat $tmp/case6.log)" = Dict ] || { echo "case6 (core): expected Dict, got:"; cat $tmp/case6.log; }

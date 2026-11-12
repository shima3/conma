#!/bin/bash
# 高速版 ConMa の機能 "bitwise32"（docs/BITWISE-PLAN-ja.md の W2、W3）：ビット演算の 7 つのプリミティブ。
# step41 と同じ式を、代わりの定義（include/fast/bitwise32.se）を入れずに、モジュールに (Uses "bitwise32") を付けて動かす。
# 高速版の VM（Rust、wasm、変換版）はプリミティブで、簡潔版の VM（Python）は test/vm.sh が bin/concise で簡潔版に
# 直したモジュール（代わりの定義を入れたもの）で、どれも step41 と同じログ（Node.js の値）になる。
# bin/concise で直したモジュールを、それぞれの VM で動かしても同じ（代わりの定義とプリミティブが同じ値になる）。
# own.se：自分で定義した名前は、直しても自分の定義のまま
. "$(dirname "$0")/../common.sh"
resolve test.se own.se
perl -pe 's/^(\(Module \(F [^)]*\))/$1 (Uses "bitwise32")/ if $. == 1' $tmp/test.module.se > $tmp/fast.module.se
"$root/bin/concise" $tmp/fast.module.se > $tmp/concise.module.se || exit 1
grep -q '(Uses' $tmp/concise.module.se && echo "bin/concise left (Uses ...)"
grep -q '(G [0-9]* "__Number_xor32__"$' $tmp/concise.module.se || echo "bin/concise did not put the definition of __Number_xor32__"
for m in fast concise
do
  for c in case1 case2 case3
  do
    PYTHONUNBUFFERED=1 ../vm.sh $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff ../step41/$c.log $tmp/$c.log
  done
done
# 自分で定義した名前は、bin/concise が代わりの定義で替えない
perl -pe 's/^(\(Module \(F [^)]*\))/$1 (Uses "bitwise32")/ if $. == 1' $tmp/own.module.se > $tmp/ownfast.module.se
PYTHONUNBUFFERED=1 ../vm.sh --main $tmp/ownfast.module.se < /dev/null > $tmp/own.log 2>&1
echo "exit: $?" >> $tmp/own.log
diff own.log $tmp/own.log

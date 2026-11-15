#!/bin/bash
# 拡張 ConMa の印 (Uses ...)（docs/FAST-PLAN-ja.md 3.4）と、空の変換器 bin/to-core（段階 F1）。
# どの VM も、機能を挙げない (Uses) は動かし、知らない機能を挙げた (Uses ...) はエラーにする（どの VM も、まだ機能を知らない）。
# bin/to-core は (Uses) を前の空白ごと消し、ほかは読んだとおりに書く。機能を挙げていればエラーにする
. "$(dirname "$0")/../common.sh"
resolve test.se
m=$tmp/test.module.se
# uses0: 1 行目の (F ...) の後に (Uses) を置く。uses1: 次の行に (Uses "if" "return") を置く
perl -pe 's/^(\(Module \(F [^)]*\))/$1 (Uses)/ if $. == 1' $m > $tmp/uses0.module.se
perl -pe 's/^(\(Module \(F [^)]*\))/$1\n  (Uses "if" "return")/ if $. == 1' $m > $tmp/uses1.module.se
{
  for c in uses0 uses1
  do
    echo "== run $c"
    PYTHONUNBUFFERED=1 ../../bin/vm $tmp/$c.module.se main < /dev/null 2>&1 | sed "s|$tmp/||g"
    echo "exit: ${PIPESTATUS[0]}"
  done
  for c in uses0 uses1
  do
    echo "== to-core $c"
    "$root/bin/to-core" $tmp/$c.module.se > $tmp/$c.core.se 2>&1
    echo "exit: $?"
    if cmp -s $m $tmp/$c.core.se; then echo "the module without (Uses)"; else cat $tmp/$c.core.se; fi
  done
  echo "== to-core from standard input"
  "$root/bin/to-core" < $tmp/uses0.module.se | cmp -s $m - && echo "the module without (Uses)"
  echo "== to-core: not a Module"
  echo '(G 0 "main")' | "$root/bin/to-core" 2>&1
  echo "exit: $?"
} > $tmp/case1.log
diff case1.log $tmp/case1.log

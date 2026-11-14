#!/bin/bash
# 値を自分で確かめる 6 つの条件分岐（test.se）。各 case を、組み込み（test.se。(Uses "control")）と ConMa 版
# （conma.se。include/ref/control.se の定義）の両方で実行して比べる。Python VM（簡潔版）では、test/vm.sh が bin/concise
# で直すので、どちらも定義になる。数でない値のエラー（case4〜6）は、組み込みでは組み込みの名前とステップの番号に
# なる（caseN.log）。ConMa 版と Python VM は、確かめるプリミティブの名前とその番号になる（caseN.python.log）
. "$(dirname "$0")/../common.sh"
CONMA_FAST=1 resolve test.se conma.se
# どちらのフロントエンドも、6 つのどれか 1 つだけを使うモジュールに (Uses "control") を付ける（bin/resolver と、
# ConMa 版の conma build）。test.se は util.se が __if_then_else__ などを使うので、そこでは確かめられない
conma="$root/vm/target/release/conma"
for p in __if_null_then_else__ __unless_null__ __Number_if_zero_then_else__ __Number_unless_zero__ __Number_if_greater__ __Number_if_greater_then_else__
do
  printf '(define main ,(v)\n  %s v)\n' $p > $tmp/one.se
  (cd $tmp && "$root/bin/includer" one.se | "$root/bin/resolver") | head -1 | grep -q '(Uses "control")$' || echo "bin/resolver did not write (Uses \"control\") for $p"
  if [ -x "$conma" ]; then
    "$conma" build $tmp/one.se | grep -q '^(Module (F [^)]*) (Uses "control")' || echo "conma build did not write (Uses \"control\") for $p"
  fi
done
if [ -x "$conma" ]; then
  (cd "$root" && "$conma" build test/fast10/test.se) | sed -e "s|\"$(cd "$root/.."; pwd -P)/|\"|g" > $tmp/built.module.se
  python3 ../selfhost_compare.py ast $tmp/test.module.se $tmp/built.module.se > /dev/null || echo "conma build made another module"
fi
"$root/bin/concise" $tmp/test.module.se > $tmp/concise.module.se || exit 1
grep -q '(G [0-9]* "__Number_if_greater_then_else__"$' $tmp/concise.module.se || echo "bin/concise did not put the definition of __Number_if_greater_then_else__"
for c in case1 case2 case3 case4 case5 case6
do
  for m in test conma
  do
    log=$c.log
    [ -f $c.python.log ] && { [ $m = conma ] || [ "${CONMA_VM:-python}" = python ]; } && log=$c.python.log
    PYTHONUNBUFFERED=1 ../vm.sh $tmp/$m.module.se $c < /dev/null > $tmp/$c.log 2>&1
    echo "exit: $?" >> $tmp/$c.log
    diff $log $tmp/$c.log || echo "$c $m"
  done
done
# 速い道と遅い道のステップの番号
if [ "$CONMA_VM" = rust ] || [ "$CONMA_VM" = wasm ]; then
  ../vm.sh $tmp/test.module.se case7 < /dev/null 2>&1 | grep '^Error in VProc' > $tmp/fast.log
  ../vm.sh -t $tmp/test.module.se case7 < /dev/null 2>&1 | grep '^Error in VProc' > $tmp/slow.log
  [ -s $tmp/fast.log ] || echo "case7: no error"
  diff $tmp/fast.log $tmp/slow.log || echo "case7: the steps differ with --trace"
fi
exit 0

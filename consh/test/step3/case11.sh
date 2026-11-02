# if 文の中の変数定義と関数定義は外に反映する。return は関数から戻る。リダイレクトは if 文全体に効く
if true; then x=1; function h () { echo "h $x"; }; fi
echo "x=$x"
h
function f (n) {
  if test $n = 0; then return 5; fi
  echo "f $n"
}
f 1
f 0
echo "f $?"
d=$1
if echo in; then echo then; fi > $d/out
cat $d/out
function g () {
  if true; then echo first; return 3; fi > $d/out2
  echo not-here
}
g
echo "g $?"
cat $d/out2
if true; then echo err 1>&2; fi 2>&1

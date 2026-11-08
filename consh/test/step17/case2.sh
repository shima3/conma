# 関数値の引数はエラー（LazyList の中にあっても）
f=function (x) { echo $x }
list=(a $f)
js '(l) => l' ${list}
echo "not reached"

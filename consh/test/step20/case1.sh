# 関数値：作った時点の変数、後で定義した関数、引数と rest、return と $?
x=1
f=function (a b . rest) { echo "a=$a b=$b x=$x rest=${rest}"; later; }
x=2
function later () { echo "later x=$x"; }
$f p q r s
$f
g=function () { return 7; }
$g
echo "status $?"
# LazyList、チャネル、$( )、変数リストを通す
l=$(echo $f; echo from-list)
${l} via-list
fs=$(echo $g; echo $g)
for h in ${fs}; do $h; echo "loop $?"; done
echo $f > (c)
$c via-capture
arr=($f other)
${arr} from-array
function apply (fn . args) { $fn ${args}; }
apply $f 1 2 3
# 関数値の中で作る関数値
mk=function (n) { inner=function (m) { echo "inner $n $m"; }; echo $inner; }
$mk 5 > (i5)
$i5 6
# パイプ、写し、既定値
$f piped | cat
k=$f
$k copy
y=${undefined-function (a) { echo "default $a"; }}
$y d
# 自分の名前の変数は、作った時点ではないので見えない
rec=function (n) { if defined rec; then echo "sees rec"; else echo "no rec"; fi; }
$rec

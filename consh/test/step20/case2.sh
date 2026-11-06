# bind：関数、関数値、組み込みコマンド、外部コマンド、bind を重ねる
function add (a b c) { echo "add $a $b $c"; }
bind add 1 > (a1)
$a1 2 3
bind $a1 9 > (a19)
$a19 8
f=function (x . r) { echo "f $x ${r}"; }
bind $f F > (bf)
$bf G H
bind echo hello > (e)
$e world
bind printf '%s-%s\n' > (p)
$p a b
# 名前は呼ぶときに探すので、後で定義した関数も見える
bind late x > (bl)
function late (v) { echo "late $v"; }
$bl y
# 同じ名前の関数と変数：bind add は関数、bind $add は変数の値
add=function (a) { echo "var add $a"; }
bind add 1 > (byname)
bind $add 1 > (byvalue)
$byname 2 3
$byvalue
# 文字列の変数は名前
n=add
bind $n 7 > (bn)
$bn 8 9
# 関数の中で bind した名前は、その関数のスコープから探す
function outer () { function helper (v) { echo "helper $v"; }; bind helper in; }
outer > (bh)
$bh
x=$(bind add q)
${x} r s

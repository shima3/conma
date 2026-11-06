# 関数値と bind も、呼ぶときの引数（bind で付けたものを含む）の数で調べる
v=function (a b) { echo "a=$a b=$b" }
$v 1 2
bind $v 1 > (w)
$w 2
$w 2 3
echo not reached

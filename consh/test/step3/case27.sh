# 残りの変数（. rest）があれば、多いのはよいが、足りなければエラー。$x は展開した語の数で数える
function f (a . rest) { echo "a=$a rest=$rest" }
x=$(echo 1; echo 2)
f $x 3
f ${x}
f
echo not reached

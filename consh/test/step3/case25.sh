# 引数の数が変数リストと違えばエラー：足りない
function f (a b) { echo "a=$a b=$b" }
f 1 2
f 1
echo not reached

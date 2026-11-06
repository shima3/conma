# 引数の数が変数リストと違えばエラー：多い（変数リストが空でも）
function f (a) { echo "a=$a" }
f 1
function g () { echo g }
g
g 1
echo not reached

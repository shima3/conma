# > (a b . rest)：標準出力の1つ目、2つ目の値を a、b に、残りの LazyList を rest に渡す
function f () { echo one; echo two; echo three; }
f > (a b . rest)
echo "a=$a b=$b rest=${rest}"
# 外部コマンドは行ごと。足りなければ空文字列と空の LazyList
printf 'x\ny\n' > (p q r . more)
echo "p=$p q=$q r=[$r] more=[${more}]"
# 値が LazyList なら、LazyList のまま渡す
l=(m n)
function g () { echo ${l}; }
g > (v)
for e in ${v}; do echo "elem $e"; done
# if 文と for 文にも使える
if true; then echo in-if; fi > (iv)
for x in 1 2 3; do echo "for $x"; done > (f1 . fr)
echo "iv=$iv f1=$f1 fr=${fr}"
# 語がすべて消えた文でも、変数は空文字列になる
none=""
$none > (empty)
echo "empty=[$empty]"

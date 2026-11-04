# < (a b . rest)：a、b の値と rest の要素を標準入力にする
x=1
l=(m n)
function g () { stdin > (s . t); echo "s=$s t=${t}"; }
g < (x . l)
cat < (x . l)
# 外部コマンドには、LazyList の値を OFS で文字列化して1行にする
OFS=,
cat < (l)

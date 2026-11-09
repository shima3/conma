# JS の値：オブジェクトは同じものが渡り、JS で変えれば $ にも表れる。関数は $f で呼べる
o = {a: 1}
o.a = 2
echo "o=$o"
p = o
p.a = 3
echo "o=$o"
add = (a, b) => Number(a) + Number(b)
$add 1 2
# 関数の写し：定義した時点の変数
q = 1
function show () { echo "q=$q" }
q = 2
show
# js 変数名= は、今までどおり
js wv= '[1, 2]'
JSON.stringify(wv)
# シェルで代入し直すと String になる
o=plain
typeof o

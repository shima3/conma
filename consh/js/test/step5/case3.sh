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
# js 変数名= も、結果を JS の値のまま入れる（数は数のまま）。$ ではリスト
js wv= '[1, 2]'
JSON.stringify(wv)
echo "wv=$wv"
js ob= '({k: 1})'
ob.k + 1
echo "ob=$ob"
# シェルで代入し直すと String になる
o=plain
typeof o
# $ は読むたびに今の値に直す（JS の文が終わった後に変えたものも）
o2 = {v: 1}
void setTimeout(() => { o2.v = 2 }, 0)
sleep 0.1
echo "o2=$o2"
# 数や真偽値は、表に入れずに値のまま持つ。JS では数のまま
k3 = 0.1 + 0.2
k3 * 10
t3 = true
echo "$k3 $t3"
typeof k3
# 代入する名前は、大域変数と同じ名前でもシェルの変数になる（大域変数は globalThis.名前 で変える）
process = 5
echo "process=$process"
typeof globalThis.process

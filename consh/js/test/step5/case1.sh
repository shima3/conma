# 判定：先頭の語がコマンドでなければ JS の文。代入はシェルの変数になり、$ で文字列やリストになる
# （名前は、PATH にあるコマンドと同じにならないように選ぶ。macOS には x と w がある）
n = 3
n * 2
echo $n
n += 1
echo "n=$n"
let m = n + 10
echo "m=$m"
# 名前= で始まる文はシェルの変数定義（String）
n=1+2
echo "n=$n"
JSON.stringify(n)
# $( ) の中の JS の文。値は出力の行のリスト
n=$(1+1)
JSON.stringify(n)
# オブジェクトは JSON、配列はリスト、undefined と null は空
o = {a: 1, b: [1, 2]}
o.a + 1
echo "o=$o"
xs = [1, 2, 3].map(i => i * 2)
for x in $xs; do echo "x=$x"; done
u = undefined
echo "u=[$u]"
# シェルの変数（String とリスト）も JS の変数として見える
name=world
console.log(`hello ${name}`)
list=(p q r)
list.join("+")
# 宣言（let、const、var、function、class）もシェルの変数になり、次の文で使える
const c1 = "const"
var v1 = "var"
let a1, a2
class Point { constructor(x) { this.x = x } }
new Point(5).x
echo "$c1 $v1 [$a1]"
let {p1, p2} = {p1: "P1", p2: "P2"}
echo "$p1 $p2"

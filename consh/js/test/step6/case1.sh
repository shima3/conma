# 算術評価：値は出さず、$? は値の真偽（真なら 0、偽なら 1）。代入は SJS変数を変える
(( n = 1 + 2 ))
echo "status $?"
(( n > 5 ))
echo "status $?"
(( p = 0 ))
echo "status $?"
(( q = "" ))
echo "status $?"
(( r = null ))
echo "status $?"
(( t = [] ))
echo "status $?"
# 算術置換：ほかの語要素とは文字列にして連結する。"..." の中でも使える
echo "n is $(( n ))"
echo a$(( n * 2 ))b
echo "$(( "q))" + n ))"
# 算術置換だけの語は、値のまま変数に入れる。$ で読むと、JS の値と同じく文字列かリストになる
v=$(( [n, n * 2, "three"] ))
for x in $v; do echo "[$x]"; done
o=$(( { a: n, b: [1, 2] } ))
echo "o: $o"
# シェル変数と SJS変数は別のもの
n=shell
echo "shell n: $n, SJS n: $(( n ))"
# 関数の本体には、if、for、return も書ける
(( sum = (a) => { let s = 0; for (const x of a) { if (x > 1) s += x } return s } ))
echo "sum: $(( sum([1, 2, 3]) ))"
# SJS変数は、シェル変数と同じく関数の中では写し
function f () { (( n = 100 )); echo "in f: $(( n ))" }
f
echo "after f: $(( n ))"
echo "in a pipeline: $(( n ))" | cat
# 複数行
(( m = [
  1,
  2
].length ))
echo "m: $(( m ))"
# console.log は標準出力に出る
(( console.log("log", n) ))
(( console.log("to a file") )) > $1/out.txt
cat $1/out.txt
# SJS関数の中の大域の名前は、呼び出したときの SJS変数（呼ぶコードに現れない名前も）
(( m = 5 ))
(( g = () => m ))
echo "g: $(( g() ))"
function callg () { (( m = 50 )); echo "g in callg: $(( g() ))" }
callg
echo "g after callg: $(( g() ))"
(( h = () => { k = 7 } ))
(( h() ))
echo "k: $(( k ))"

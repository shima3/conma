# 値：$? は値の真偽。算術置換は、ほかの語要素とは文字列にして連結し、だけの語は値のまま変数に入れる
(( n = 1 + 2 ))
echo "status $? n is $(( n ))"
(( n > 5 ))
echo "status $?"
echo "a$(( n * 2 ))b" "$(( `q))${n}` ))"
v=$(( [n, n * 2, "three"] ))
for x in $v; do echo "[$x]"; done
o=$(( { a: n, b: [1, 2] } ))
echo "o: $o"
# シェル変数と SJS変数は別。$名前 でシェル変数を読む（演算子の隣は数）
n=shell
x=7
echo "shell n: $n, SJS n: $(( n )), $(( $x + 1 )) $(( `${$x}!` ))"
# 関数とメソッド、文
(( fact = (k) => k <= 1 ? 1 : k * fact(k - 1) ))
(( sum = (a) => { let s = 0; for (const e of a) { if (e > 1) s += e } return s } ))
echo "$(( fact(10) )) $(( sum([1, 2, 3]) )) $(( [3, 1, 2].map((e) => e * 10).join("-") ))"
(( sw = (k) => { switch (k) { case 1: return "one"; default: return "other" } } ))
(( tr = () => { try { throw "t" } catch (e) { return `caught ${e}` } } ))
echo "$(( sw(1) )) $(( sw(2) )) $(( tr() ))"
(( [p, q = 5, ...r] = [1, undefined, 3, 4] ))
echo "$(( p )) $(( q )) $(( r ))"
# SJS変数は、関数とパイプラインで写し。SJS関数の中の大域の名前は、呼び出したときの SJS変数
(( m = 5 ))
(( g = () => m ))
function inner () { (( m = 50 )); echo "in inner: $(( m )) $(( g() ))" }
inner
echo "after inner: $(( m ))"
echo "in a pipeline: $(( m ))" | cat
# console.log は標準出力に出る
(( console.log("log", n, [1, 2]) ))
(( console.log("to a file") )) > $1/out.txt
cat $1/out.txt
# SJS関数をコマンドとして呼ぶ：戻り値は標準エラー、$? は戻り値の真偽
add=$(( (a, b) => Number(a) + Number(b) ))
$add 1 2 2> (c)
echo "c=$c status $?"
$(( (s) => s.toUpperCase() )) hello 2>&1
(( isBig = (k) => Number(k) > 10 ))
big=$(( isBig ))
if $big 50 2>/dev/null; then echo big; fi
if $big 5 2>/dev/null; then echo big; else echo small; fi

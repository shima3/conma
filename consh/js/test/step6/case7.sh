# 段階 A2：$名前 でシェル変数を読む（consh/SAFEJS-PLAN-ja.md の 2.2 の 7）
n=3
echo "a: $(( $n + 1 ))"
echo "b: $(( `${$n}x` ))"
z=007
echo "c: $(( $z + 1 )) $(( `${$z}` )) $(( ($z) * 2 )) $(( -$z ))"
(( $n < 10 )); echo "lt status $?"
(( $n === 3 )); echo "=== number status $?"
(( k = $n )); echo "k: $(( typeof k ))"
xs=(1 2 3)
echo "len: $(( $xs.length ))"
export E=env
echo "env: $(( $E ))"
s=abc
(( $s + 1 )); echo "not a number status $?"
(( $undefinedvar )); echo "undefined status $?"
(( $n = 5 )); echo "assign status $?"
(( (() => { let $x = 1 })() )); echo "declare status $?"
# SJS関数の中の $名前 は、呼び出したときのシェル変数を読む
(( g = () => $n * 2 ))
echo "g: $(( g() ))"
function inner () { n=100; echo "g in inner: $(( g() ))"; }
inner
# SJS関数をコマンドとして呼ぶ（2.2 の 3）。戻り値は標準エラー、$? は戻り値の真偽
add=$(( (a, b) => Number(a) + Number(b) ))
$add 1 2 2> (c)
echo "status $? c=$c"
$add 1 2 2>&1 | cat
$(( (x) => x * 2 )) 21 2>&1
(( isBig = (x) => Number(x) > 10 ))
f=$(( isBig ))
if $f 50 2>/dev/null; then echo big; fi
if $f 5 2>/dev/null; then echo big; else echo small; fi
$(( (x) => { m = x } )) hello
echo "status $? m: $(( m ))"
$(( (l) => l.length )) ${xs} 2>&1
$(( (...a) => a.length )) $xs 2>&1
$(( () => { throw "x" } ))
echo "throw status $?"

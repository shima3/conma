# 演算の型（consh/SAFEJS-PLAN-ja.md の 3.1）：+ は数どうしだけを足し、算術とビット演算は数だけ、大小の比較は数どうしか文字列どうし
echo "1: $(( 1 + 2 * 3 ** 2 ))"
(( "a" + "b" )); echo "string + status $?"
(( x = "3" ))
(( x + 1 )); echo "string var + status $?"
(( x * 2 )); echo "string var * status $?"
(( -x )); echo "unary - status $?"
(( x++ )); echo "++ status $?"
(( x += 1 )); echo "+= status $?"
(( o = {n: "1"} ))
(( o.n += 1 )); echo "member += status $?"
echo "join: $(( `a${1 + 1}b` )) $(( ["a", "b"].join("") ))"
echo "compare: $(( "b" < "c" )) $(( 2 < 10 ))"
(( "b" < 1 )); echo "mixed < status $?"
(( null | 0 )); echo "| status $?"
# 数どうしなら、複合の代入や ++ も使える
(( i = 0 )); (( i++ )); (( ++i )); (( i -= 1 ))
(( o = {n: 1} )); (( o.n += 5 )); (( o.n++ )); (( o["n"] *= 2 ))
echo "i: $(( i )) o.n: $(( o.n ))"
# 関数の本体の文
(( fact = (n) => { if (n <= 1) { return 1 } else { return n * fact(n - 1) } } ))
(( loop = (n) => { let s = 0; for (let k = 0; k < n; k++) { if (k % 2) continue; s += k } return s } ))
(( sw = (v) => { switch (v) { case 1: return "one"; default: return "other" } } ))
(( tc = () => { try { throw "e" } catch (e) { return `caught ${e}` } finally { } } ))
(( dw = () => { let k = 0; do { k++ } while (k < 3); return k } ))
(( fo = (o) => { let r = []; for (const k in o) r.push(k); for (const [k, v] of Object.entries(o)) r.push(v); return r.join("") } ))
echo "$(( fact(10) )) $(( loop(10) )) $(( sw(1) )) $(( sw(2) )) $(( tc() )) $(( dw() )) $(( fo({x: "1", y: "2"}) ))"
# 分割代入、既定値、残り、メソッド、?. と ??
echo "$(( ((a, b = 10, ...r) => a + b + r.length)(1, 2, 3) )) $(( (({a, b: [c, d = 4]}) => a + c + d)({a: 1, b: [2]}) ))"
(( [a1, b1] = [1, 2] )); (( ({p1, p2 = 5} = {p1: 1}) ))
(( m = {f(y) { return y * 2 }} ))
echo "$(( a1 + b1 )) $(( p1 + p2 )) $(( m.f(4) )) $(( m?.missing?.x ?? "none" ))"
# 構文エラー
(( (1 + ) )); echo "syntax status $?"
(( ({ get x() { return 1 } }) )); echo "getter status $?"

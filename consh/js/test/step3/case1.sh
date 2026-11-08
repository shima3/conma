# イテレータ（ジェネレータを含む、配列と文字列以外の反復可能なもの）の結果は、要素ごとに値1つ。
# 要素は配列の要素と同じく変換する（配列は LazyList、null と undefined は空文字列）
js '(function* () { yield "a"; yield ["b", "c"]; yield 3; yield null })()'
# 関数がジェネレータを返すときも
js 'function* (n) { for (let i = 1; i <= Number(n); i++) yield "line " + i }' 3
js 'new Set(["s1", "s2", "s1"])'
js 'new Map([["k1", "v1"], ["k2", "v2"]])'
# 非同期のジェネレータは、要素ができるのを待つ
js '(async function* () { for (const d of [40, 10]) { await new Promise(r => setTimeout(r, d)); yield "after " + d } })()'
# 途中の例外は、それまでの要素を出した後、$? を 1 にする
js '(function* () { yield 1; throw new Error("in the middle") })()'
echo "status $?"
# 空のイテレータ
js '[].values()'
echo "status $?"

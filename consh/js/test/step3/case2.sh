# 必要な分だけ要素を作る：変数リストと、読み手が先に終わるパイプ（終わらないジェネレータでも終わる）。
# console.log の出力も標準出力の要素なので、変数リストに入る
js '(function* () { let i = 0; while (true) { console.log("make " + i); yield i++ } })()' > (a b c)
echo "a=$a b=$b c=$c"
function first () { stdin > (x); echo "first: $x" }
js 'function* () { let i = 0; while (true) yield "n" + i++ }' | first
# 止めるときは、イテレータの return() を呼ぶ（finally の console の出力は、ホストの出力に直接書く）
js 'function* () { try { let i = 0; while (true) yield i++ } finally { console.error("finally ran") } }' | first
echo "after the pipes"
# ジェネレータも stdin を読める
function gen () { echo one; echo two; echo three }
gen | js 'async function* () { for await (const l of stdin) yield l.toUpperCase() }'

# 式として読めないコードは、async 関数の本体として動かし、return の値を結果にする
js 'let x = 1; return x + 1'
js r= 'const parts = ["a", "b"]; return parts.join("-")'
echo "r=$r"
# 本体の中では、引数は args（配列）
js 'const a = []; for (const s of args) a.push(s.toUpperCase()); return a' p q
# stdin と await も使える
function nums () { echo 1; echo 2; echo 3 }
nums | js 'let s = 0; for await (const l of stdin) s += Number(l); return s'
js 'await new Promise(r => setTimeout(r, 10)); return "after await"'
# console も
js 'console.log("from a body"); return "value"'
# return のない本体は、何も出さない（$? は 0）
js 'let x = 1; x + 1'
echo "status $?"
# ジェネレータを返せば、要素ごとに出す
js 'return (function* () { yield "g1"; yield "g2" })()'
# 式として読めるコードは、今までどおり式（{a: 1} はオブジェクト）
js '{a: 1}'

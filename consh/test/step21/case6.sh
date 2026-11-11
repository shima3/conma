# SJS の言語の残り：async ジェネレータと for await、タグ付きテンプレート、配列の穴と length
(( ticks = async function* (n) { for (let i = 1; i <= n; i++) { await null; yield i * 10 } } ))
(( sum = async (g) => { let s = 0; for await (const x of g) s += x; return s } ))
echo "sum: $(( sum(ticks(3)) ))"
(( g = ticks(2) ))
echo "first: $(( g.next().then((r) => r.value) )), second: $(( g.next().then((r) => r.value) )), done: $(( g.next().then((r) => r.done) ))"
echo "typeof $(( typeof g )), $(( `${g}` ))"
echo "$(( String.raw`a\tb${1 + 1}` )) $(( ((s, ...v) => `${s.raw.join("|")} ${v.join(",")}`)`x${1}y${2}` ))"
(( a = [1, , 3] ))
echo "$(( a.map((x) => x * 2).length )) $(( JSON.stringify(a.filter(() => true)) ))"
(( a.length = 1 ))
echo "$(( JSON.stringify(a) ))"
(( () => { let z = 1; let z = 2 } ))
echo "status $?"

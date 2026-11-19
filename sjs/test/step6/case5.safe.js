// for...of を途中で抜けたときと、分割代入の後に、ジェネレータを閉じる（return()）
function* g(name) { try { yield 1; yield 2; yield 3 } finally { console.log(`closed ${name}`) } }
for (const x of g("break")) { if (x === 2) break; console.log("got", x) }
const first = () => { for (const x of g("return")) return x }
console.log("first", first())
try { for (const x of g("throw")) throw "oops" } catch (e) { console.log("caught", e) }
for (const x of g("end")) {}
const [a, b] = g("destructure")
console.log(a, b)
const [c, ...rest] = g("rest")
console.log(c, JSON.stringify(rest))
outer: for (const i of [1, 2]) { for (const x of g(`label ${i}`)) continue outer }
function* naturals() { let n = 0; while (true) yield n++ }
const [z0, z1, z2] = naturals()
console.log(z0, z1, z2)
let s = 0
for (const v of [1, 2, 3]) s += v
for (const ch of "ab") s += ch.length
console.log(s)

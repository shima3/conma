// for、for...of、for...in、break、continue、while
let total = 0
for (let i = 0; i < 10; i++) { if (i === 3) continue; if (i === 7) break; total += i }
console.log(total)
const fs = []
for (let i = 0; i < 3; i++) fs.push(() => i)
console.log(fs.map((f) => f()))
for (const x of [10, 20]) console.log("of", x)
for (const key in { p: 1, q: 2 }) console.log("in", key)
for (const ch of "hey") console.log(ch)
for (const i in ["a", "b"]) console.log("index", i, typeof i)
let w = 0
while (true) { w++; if (w > 4) break }
console.log(w)
const found = (list, x) => { for (const v of list) { if (v === x) { return "found" } } return "missing" }
console.log(found([1, 2], 2), found([1, 2], 3))

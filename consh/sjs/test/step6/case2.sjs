// for (;;) の最初の部分の分割代入
const out = []
for (let [a, b] = [0, 10]; a < b; a += 3, b -= 1) out.push(`${a}-${b}`)
for (let { x, y } = { x: 1, y: 3 }, z = 0; x < y; x++) out.push(x + z)
const fs = []
for (let [i] = [0]; i < 3; i++) fs.push(() => i)
console.log(out.join(" "), fs.map((f) => f()).join(","))
// continue の後も、本体で変えた回ごとの変数を、ループの変数に写し戻す
const c1 = []; for (let i = 0; i < 6; i++) { if (i === 1) { i += 2; continue } c1.push(i) }; console.log(JSON.stringify(c1))
const o2 = []
outer: for (let i = 0; i < 6; i++) { for (const j of [0]) { if (i === 1) { i += 2; continue outer } } o2.push(i) }
const fs2 = []
for (let i = 0; i < 3; i++) { fs2.push(() => i); if (i === 0) continue }
console.log(JSON.stringify(o2), fs2.map((f) => f()).join(","))

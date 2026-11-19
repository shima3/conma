// ラベル（break、continue、finally を通るもの）
const out = []
outer: for (let i = 0; i < 4; i++) {
  for (let j = 0; j < 4; j++) {
    if (j === 2) continue outer
    if (i === 3) break outer
    out.push(`${i}${j}`)
  }
}
console.log(out.join(" "))
let n = 0
loop: while (true) { n++; if (n > 5) break loop }
block: { console.log("in block"); if (n > 0) break block; console.log("not reached") }
const f = () => {
  outer: for (const x of [1, 2, 3]) {
    try { for (const y of [1, 2]) { if (x === 2) continue outer; if (x === 3) break outer; console.log("xy", x, y) } }
    finally { console.log("finally", x) }
  }
  return "done"
}
console.log(n, f())
let k = 0
again: do { k++; if (k < 3) continue again } while (k < 5)
console.log(k)

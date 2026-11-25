// Stage H0 of docs/dev/CONJS-HOST-PLAN.md: the JavaScript that the translator would write for the loop of
// conjs/bench/hof.con.js, moved into a function round(r), written by hand, in Node.js.
//   node hof_host.js VARIANT N
// VARIANT:
//   node  hof.con.js as it is (Node.js running the ConJS program; the loop at the top level)
//   fn    the body of the loop as a function round(r), with a counter of fuel in each loop (section 6)
//   rt    the same, with + through rt.add (the translator does not know the types; section 5) and sort through a
//         port of the merge sort of the runtime (Sjs_msort of sjs/include/sjs_rt.se; section 5, Sorting)
//   gen   round as a generator (a waiting host function) that yields when its fuel runs out
const [variant, nArg] = process.argv.slice(2)
const n = Number(nArg)
const F = { n: 1e6, outs: 0 }
function out() { F.outs++; F.n = 1e6 }
const rt = {
  add(a, b) {
    if (typeof a !== "number" || typeof b !== "number") throw new TypeError("+ joins numbers only")
    return a + b
  },
  // stable merge sort, top-down, as Sjs_msort
  sort(a, cmp) {
    const m = (l) => {
      if (l.length < 2) return l
      const h = l.length >> 1, x = m(l.slice(0, h)), y = m(l.slice(h)), r = []
      let i = 0, j = 0
      while (i < x.length && j < y.length) r.push(cmp(y[j], x[i]) < 0 ? y[j++] : x[i++])
      while (i < x.length) r.push(x[i++])
      while (j < y.length) r.push(y[j++])
      return r
    }
    const s = m(a)
    for (let i = 0; i < s.length; i++) a[i] = s[i]
    return a
  },
}
function roundFn(r) {
  const a = []
  for (let i = 0; i < 100; i++) { if (--F.n < 0) out(); a.push((i * 37 + r) % 101) }
  const b = a.map(x => x * 2).filter(x => x % 3 !== 0)
  b.sort((x, y) => x - y)
  return b.reduce((s, x) => s + x, 0) + b[0] + b[b.length - 1]
}
function roundRt(r) {
  const a = []
  for (let i = 0; i < 100; i++) { if (--F.n < 0) out(); a.push((i * 37 + r) % 101) }
  const b = a.map(x => x * 2).filter(x => x % 3 !== 0)
  rt.sort(b, (x, y) => x - y)
  return rt.add(rt.add(b.reduce((s, x) => rt.add(s, x), 0), b[0]), b[b.length - 1])
}
function* roundGen(r) {
  const a = []
  for (let i = 0; i < 100; i++) { if (--F.n < 0) { yield "fuel"; F.n = 1e6 } a.push((i * 37 + r) % 101) }
  const b = a.map(x => x * 2).filter(x => x % 3 !== 0)
  b.sort((x, y) => x - y)
  return b.reduce((s, x) => s + x, 0) + b[0] + b[b.length - 1]
}
let acc = 0
switch (variant) {
  case "node":
    for (let r = 0; r < n; r++) {
      const a = []
      for (let i = 0; i < 100; i++) a.push((i * 37 + r) % 101)
      const b = a.map(x => x * 2).filter(x => x % 3 !== 0)
      b.sort((x, y) => x - y)
      acc = acc + b.reduce((s, x) => s + x, 0) + b[0] + b[b.length - 1]
    }
    break
  case "fn": for (let r = 0; r < n; r++) acc = acc + roundFn(r); break
  case "rt": for (let r = 0; r < n; r++) acc = rt.add(acc, roundRt(r)); break
  case "gen":
    for (let r = 0; r < n; r++) {
      const g = roundGen(r)
      let x = g.next()
      while (!x.done) x = g.next()
      acc = acc + x.value
    }
    break
  default: throw new Error(`unknown variant ${variant}`)
}
console.log(acc)

// generators: next, for...of, spread, return value, arguments
function* count(n) {
  for (let i = 0; i < n; i++) yield i
  return "end"
}
const g = count(2)
console.log(typeof g, JSON.stringify(g.next()), JSON.stringify(g.next()), JSON.stringify(g.next()), JSON.stringify(g.next()))
for (const x of count(3)) console.log("of", x)
console.log([...count(4)], Math.max(...count(5)))
const [a, b] = count(10)
console.log(a, b)
// values sent with next
function* echo() {
  let got = yield "first"
  while (true) got = yield `got ${got}`
}
const e = echo()
console.log(e.next("ignored").value, e.next(1).value, e.next(2).value)
// endless generators and break
function* naturals() { let n = 1; while (true) yield n++ }
let sum = 0
for (const n of naturals()) { if (n > 100) break; sum += n }
console.log(sum)
// yield*
function* both() { const r = yield* count(2); yield r; yield* ["x", "y"]; yield* "ab" }
console.log([...both()].join(" "))
// a generator method and a generator expression
const o = { *pairs() { yield [1, 2]; yield [3, 4] } }
for (const [p, q] of o.pairs()) console.log(p + q)
const h = function* () { yield 1 }
console.log([...h()].length, String(h()))

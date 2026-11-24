// Host functions with arrays, objects, closures and console.log (docs/dev/CONJS-HOST-PLAN.md, stage H3b). What a
// host function changes in its arguments is written back only when it returns; when it cannot go on, nothing is
// written back and the ConMa version runs from the start, so the log is the same on every VM.
function sum(a) { let t = 0; for (const x of a) t += x; return t }
function double(a) { return a.map((x) => x * 2) }
function evens(a) { return a.filter((x) => x % 2 === 0) }
function total(a) { return a.reduce((s, x) => s + x, 0) }
function sorted(a) { return a.slice().sort((x, y) => x - y) }
function sortInPlace(a) { a.sort((x, y) => y - x); return a.length }
function grow(a, x) { a.push(x); a[0] = "first"; return a.length }
function point(x, y) { return { x, y, sum: x + y, tags: ["p", "q"] } }
function move(p, dx) { p.x += dx; p.moved = true; return p }
function names(o) { return Object.keys(o).join("+") }
function same(a, b) { return a === b }
function nest(a) { const inner = [a, a]; return [inner, inner] }
function say(n) { for (let i = 0; i < n; i++) console.log(`line ${i}`); return n }
function sayThenFail(a) { console.log("never printed by the host"); a.push(99); return a.length + "x" }
function words(s) { return s.split(" ").map((w) => w.toUpperCase()).join("-") }
function nulls(a) { return [a, null, undefined].map((x) => x === null) }
function find(a) { return `${a.find((x) => x > 2)} ${a.findIndex((x) => x > 2)} ${a.some((x) => x > 3)} ${a.every((x) => x > 0)}` }
function counts(s) {
  const c = {}
  for (const ch of s) c[ch] = (c[ch] === undefined ? 0 : c[ch]) + 1
  return c
}
function keep(f, a) { return a.map(f) }
function inc(x) { return x + 1 }
function useInc(a) { return a.map(inc) }
function cls(p) { return p }

const a = [3, 1, 2]
console.log(sum(a), double(a), evens([1, 2, 3, 4]), total(a), sorted(a), a)
console.log(sortInPlace(a), a)
const b = [1, 2]
console.log(grow(b, 3), b)
const p = point(1, 2)
console.log(p, move(p, 10), p, names(p))
console.log(same(p, p), same(p, point(1, 2)), same(a, a))
const n = nest([1])
console.log(n, n[0] === n[1], n[0][0] === n[0][1])
console.log(say(3))
const c = [1]
try { console.log(sayThenFail(c)) } catch (e) { console.log(String(e)) }
console.log(c)
console.log(words("hello big world"), nulls(0), find([1, 2, 3, 4]))
console.log(counts("banana"))
console.log(keep((x) => x * 10, [1, 2]), useInc([1, 2]))
class P { constructor() { this.v = 1 } }
const q = new P()
console.log(cls(q) === q, cls(q))

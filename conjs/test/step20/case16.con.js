// Generator functions on the host (stage H8 of docs/dev/CONJS-HOST-PLAN.md, section 36): the generator object is the
// one of ConMa, so typeof, console.log, next, return and throw are the same; for...of and yield* in a host function
// step through the generator on the host. The log is the same on every VM, and as Node.js prints.
function* range(n) {
  for (let i = 0; i < n; i++) yield i
}
function sum(n) {
  let s = 0
  for (const x of range(n)) s += x
  return s
}
console.log("sum", sum(100))
let top = 0
for (const x of range(5)) top += x
console.log("top", top)
const g = range(2)
console.log(typeof g, g)
console.log(g.next(), g.next(), g.next(), g.next())
console.log([...range(4)])
const [a, b] = range(10)
console.log(a, b)

// what next(x) gives to yield, and the return value
function* talk() {
  const x = yield "first"
  const y = yield `got ${x}`
  return `end ${y}`
}
const t = talk()
console.log(t.next("lost"), t.next(1), t.next(2), t.next(3))

// return(v) runs finally and ends; throw(e) is caught inside, or leaves
function* guarded() {
  try {
    yield 1
    yield 2
  } finally {
    console.log("finally")
  }
}
const r = guarded()
console.log(r.next(), r.return(9), r.next())
function* catching() {
  for (;;) {
    try {
      yield "waiting"
    } catch (e) {
      console.log("caught", e.message)
      return "done"
    }
  }
}
const c = catching()
console.log(c.next(), c.throw(new Error("boom")), c.next())
const d = range(3)
d.next()
try { d.throw(new Error("out")) } catch (e) { console.log("left", e.message) }
console.log(d.next())

// an endless generator left by break in a host function, which closes it
function* naturals() {
  let i = 0
  try {
    for (;;) {
      yield i
      i++
    }
  } finally {
    console.log("closed at", i)
  }
}
function firstOver(limit) {
  for (const x of naturals()) if (x * x > limit) return x
  return -1
}
console.log("first", firstOver(50))

// yield*: an array, a string, a host generator with its return value, a generator of ConMa given as an argument (a
// label keeps it in ConMa), with its return value
function* inner() {
  yield "i1"
  yield "i2"
  return "inner's"
}
function* conma() {
  outer: for (let i = 0; i < 2; i++) { yield `c${i}`; continue outer }
  return "conma's"
}
function* outer(cg) {
  yield* [1, 2]
  yield* "ab"
  const v = yield* inner()
  yield v
  const w = yield* cg
  yield w
}
function listOf(cg) {
  const l = []
  for (const x of outer(cg)) l.push(x)
  return l
}
console.log(listOf(conma()))
console.log([...outer(conma())])

// a tree walked by a recursive generator
function* walk(node) {
  if (node === null) return
  yield* walk(node.left)
  yield node.value
  yield* walk(node.right)
}
function leaf(v) { return { left: null, value: v, right: null } }
const tree = { left: { left: leaf(1), value: 2, right: leaf(3) }, value: 4, right: leaf(5) }
function inOrder(t) {
  const out = []
  for (const v of walk(t)) out.push(v)
  return out.join(" ")
}
console.log("tree", inOrder(tree))
for (const v of walk(tree)) console.log("node", v)

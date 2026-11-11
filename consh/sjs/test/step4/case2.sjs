// exceptions and generators
function* guarded() {
  try {
    yield 1
    yield 2
  } catch (e) {
    console.log("caught in the generator:", e)
    yield "after catch"
  } finally {
    console.log("finally in the generator")
  }
  yield "end"
}
let g = guarded()
console.log(g.next().value, g.throw("boom").value, g.next().value, g.next().value, g.next().done)
// return() runs finally, skips catch, and ends the generator
g = guarded()
g.next()
console.log(JSON.stringify(g.return(42)), JSON.stringify(g.next()))
// return() before the start, throw() at the end
g = guarded()
console.log(JSON.stringify(g.return(7)))
try { g.throw("late") } catch (e) { console.log("rethrown:", e) }
// an error in the body ends the generator and reaches the caller of next
function* failing() { yield 1; null.x; yield 2 }
g = failing()
g.next()
try { g.next() } catch (e) { console.log(e.message) }
console.log(JSON.stringify(g.next()))
// a generator that calls next on itself
let me
function* reentrant() { me.next() }
me = reentrant()
try { me.next() } catch (e) { console.log(e.message) }
// try in the caller around a generator that throws inside for...of
function* bad() { yield 1; throw new_error }
try { for (const x of bad()) console.log("got", x) } catch (e) { console.log(`${e}`) }
// return in a generator with finally
function* early() { try { yield 1; return 2 } finally { console.log("cleanup") } }
console.log(JSON.stringify([...early()]))
const ee = early()
console.log(JSON.stringify(ee.next()), JSON.stringify(ee.next()))
// nested generators, each with its own handlers
function* inner() { try { yield "i1"; yield "i2" } finally { console.log("inner done") } }
function* outer() { try { for (const x of inner()) yield `o(${x})` } catch (e) { yield "no" } }
console.log([...outer()].join(" "))
// uncaught at the end
function* last() { yield 1; throw "uncaught from a generator" }
for (const x of last()) {}

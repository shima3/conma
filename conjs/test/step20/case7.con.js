// Calling ConMa back from host functions, and the slow paths in the VM after it (docs/dev/CONJS-HOST-PLAN.md,
// stage H4c). square uses **, which the host does not take, so it stays a function of the VM; host functions that
// call it call the VM back. After a call back, the host cannot run the function again in ConMa, so what it cannot
// do itself it asks the VM to do. The log is the same on every VM.
function square(x) { return x ** 2 }
function shout(s) { console.log(`shout ${s}`); return s.toUpperCase() }
function apply(f, x) { return f(x) }
function mapAll(a, f) { return a.map(f) }
function logged(a) { console.log("before"); const r = a.map(shout); console.log("after"); return r }
function afterCall(s) { const n = square(2); return `${s.toUpperCase()} ${n} ${s.length} ${"日本".at(-1)} ${1.5 + n}` }
function mutateIn(a, f) { a.push(1); const r = f(a); a.push(2); return r }
function count(a) { return a.length }
function typeErr(x) { const n = square(x); return n + "!" }
function catchErr(x) { const n = square(x); try { return n + "!" } catch (e) { return `caught ${e.message}` } }
function callbackThrows(f) { try { return f() } catch (e) { return `caught ${e}` } }
function thrower() { throw "boom" }
function uncaughtCallback(f) { const r = f(); return r }
class Err extends Error {}
function viaOpaque(e) { return e.message.toUpperCase() }

console.log(apply(square, 3), mapAll([1, 2, 3], square), mapAll([1, 2], (x) => x * 10))
console.log(logged(["a", "b"]))
console.log(afterCall("héllo"))
const arr = []
console.log(mutateIn(arr, count), arr)
try { console.log(typeErr(2)) } catch (e) { console.log(String(e)) }
console.log(catchErr(2))
console.log(callbackThrows(thrower))
try { uncaughtCallback(thrower) } catch (e) { console.log("from the callback:", e) }
console.log(viaOpaque(new Err("inner")))
console.log(typeErr(3))

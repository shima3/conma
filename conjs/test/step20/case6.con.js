// new, instanceof, throw and try on the host (docs/dev/CONJS-HOST-PLAN.md, stage H4b). The log is the same on every VM.
class Pt {
  z = 0
  constructor(x, y) { this.x = x; this.y = y }
  add(o) { return new Pt(this.x + o.x, this.y + o.y) }
}
class Pt3 extends Pt {
  tag = "3d"
  constructor(x, y, z) { super(x, y); this.z = z }
  add(o) { return new Pt3(this.x + o.x, this.y + o.y, this.z + o.z) }
}
class Odd { constructor() { return [1, 2] } }
function make(n) { const ps = []; for (let i = 0; i < n; i++) ps.push(new Pt(i, i * 2)); return ps }
function check(p) { return `${p instanceof Pt} ${p instanceof Pt3} ${[] instanceof Pt}` }
function safeDiv(a, b) { try { if (b === 0) throw "zero"; return a / b } catch (e) { return `caught ${e}` } finally { console.log("finally") } }
function odd() { return new Odd() }
console.log(make(3), new Pt(1, 2).add(new Pt(3, 4)), new Pt3(1, 1, 1).add(new Pt3(1, 2, 3)))
console.log(check(new Pt3(1, 2, 3)), check(new Pt(0, 0)), check({}))
console.log(safeDiv(1, 2), safeDiv(1, 0), odd())
const q = make(2)[1]
console.log(q.add(q), q instanceof Pt)
function nested(n) {
  const log = []
  try {
    try { if (n > 0) throw { code: n }; log.push("no throw") }
    catch (e) { log.push(`inner ${e.code}`); throw e }
    finally { log.push("inner finally") }
  } catch (e) { log.push(`outer ${e.code}`) }
  return log.join(",")
}
function uncaught(n) { if (n > 1) throw new Pt(n, n); return n }
function runtimeError(o) { try { return o.x.y } catch (e) { return `caught ${e instanceof TypeError}` } }
function mutateThenThrow(a) { a.push(1); throw "stop" }
console.log(nested(0), nested(2))
try { uncaught(5) } catch (e) { console.log("from uncaught:", e) }
console.log(uncaught(1), runtimeError({}), runtimeError({ x: { y: 3 } }))
const arr = []
try { mutateThenThrow(arr) } catch (e) { console.log(e, arr) }

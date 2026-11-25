// Host functions in a program with import and export (docs/dev/CONJS-HOST-PLAN.md, stage H6b): each file has its
// own module of the host. The log is the same on every VM.
import { scale, sleep, ticks, Pt as LibPt, makePt } from "./h6lib.con.js"

// a host function of this file that calls an imported function: it stays in ConMa (imports are not on the host)
function twice(a) { return scale(a, 2) }
function sum(a) { let s = 0; for (const x of a) s = s + x; return s }
console.log(scale([1, 2, 3], 10), twice([4]), sum([5, 6]))

// an imported function named sleep is not the sleep of ConJS
function callSleep() { return sleep(5) }
console.log(callSleep())

// a top-level loop that reads an imported variable runs in ConMa; one that reads only this file's runs on the host
let a = 0
for (let i = 0; i < 2; i++) a = a + ticks
let b = 0
for (let i = 0; i < 4; i++) b = b + sum([i, i])
console.log(a, b, ticks)

// a class with the same name as a class of the imported file is not a class of the host (instances are found by name)
class Pt {
  constructor(y) { this.y = y }
  show() { return `main Pt ${this.y}` }
}
function showAll(p, q) { return `${p.show()} / ${q.show()}` }
console.log(showAll(new Pt(1), makePt(2)), new LibPt(3).show())

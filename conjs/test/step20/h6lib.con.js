// The file that case10.con.js imports (stage H6b): host functions and a top-level loop in an imported file.
export function scale(a, k) { return a.map((x) => x * k) }
export function sleep(ms) { return `not the sleep of ConJS: ${ms}` }
export let ticks = 0
for (let i = 0; i < 3; i++) ticks = ticks + i
export class Pt {
  constructor(x) { this.x = x }
  show() { return `lib Pt ${this.x}` }
}
export function makePt(x) { return new Pt(x) }

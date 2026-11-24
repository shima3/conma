// Methods of classes on the host (docs/dev/CONJS-HOST-PLAN.md, stage H4a). The log is the same on every VM.
class Vec {
  constructor(x, y) { this.x = x; this.y = y }
  len2() { return this.x * this.x + this.y * this.y }
  add(o) { this.x += o.x; this.y += o.y; return this }
  scaled(k) { return new Vec(this.x * k, this.y * k) }
  show() { return `(${this.x}, ${this.y})` }
  twice() { return this.add(this).show() }
  sumWith(list) { let t = 0; for (const v of list) t += v.len2(); return t + this.len2() }
  shout() { console.log(`vec ${this.show()}`); return this.len2() }
}
class Vec3 extends Vec {
  constructor(x, y, z) { super(x, y); this.z = z }
  len2() { return this.x * this.x + this.y * this.y + this.z * this.z }
  pairs(ps) { let t = 0; for (const [a, b] of ps) t += a.len2() * b; return t }
  root() { return Math.sqrt(this.len2()) + Math.max(this.x, this.y, this.z) + Math.floor(Math.PI) }
}
class Counter {
  constructor() { this.n = 0 }
  hit() { this.n++; return this.n }
  get() { return this.n }
}
function useMethodValue(v) { const f = v.show; return f() }

const a = new Vec(1, 2)
const b = new Vec(3, 4)
console.log(a.len2(), a.add(b) === a, a.show(), b.show(), a.twice())
console.log(a.scaled(2).show(), a.sumWith([b, b]))
console.log(a.shout())
const c = new Vec3(1, 2, 2)
console.log(c.len2(), c.show(), c.add(a).show(), c.pairs([[c, 2], [a, 1]]), c.root())
const k = new Counter()
console.log(k.hit(), k.hit(), k.get(), k.n)
console.log(useMethodValue(b))
console.log(a, c)

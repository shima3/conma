// Stage H0 of docs/dev/CONJS-HOST-PLAN.md: the JavaScript that the translator would write for advance of
// conjs/bench/nbody_class.con.js, written by hand, in Node.js.
//   node nbody_host.js VARIANT N
// VARIANT:
//   node    the classes of nbody_class.con.js as they are (Node.js running the ConJS program)
//   fuel    the same, with a counter of fuel in the loop of advance (section 6)
//   rt      fields read and written through rt.get and rt.set, methods called through rt.call (the translator does
//           not know the types nor the classes; section 5)
//   rt-get  fields through rt.get and rt.set, but methods called directly (b1.pull(b2, dt))
//   rt-call methods through rt.call, but fields read and written directly
//   inline  the checks of rt.get, rt.set and rt.call written in place, with the names of the fields and methods
//           written as they are (o == null ? rt.fail(...) : o.x), so that the engine sees ordinary property access
//   gen     advance as a generator (a waiting host function) that yields when its fuel runs out
//   gen-all advance, pull and move all generators, called with yield* (every function a waiting host function)
//   copy    advance(dt, 1) called N times, each call copying the system from a form like the VM's and back (3.2)
// It prints the energy before and after, as nbody_class.con.js does.
const fs = require("fs")
const path = require("path")
const src = fs.readFileSync(path.join(__dirname, "..", "nbody_class.con.js"), "utf8")
const classes = src.slice(0, src.indexOf("const system = new System()"))
const { Body, System } = new Function(`${classes}; return { Body, System }`)()
const [variant, nArg] = process.argv.slice(2)
const n = Number(nArg)

// Fuel: a counter that the translator puts at the start of each loop (section 6)
const F = { n: 1e6, outs: 0 }
function out() { F.outs++; F.n = 1e6 }

const rt = {
  get(o, k) { if (o === null || o === undefined) throw new TypeError(`Cannot read properties of ${o} (reading '${k}')`); return o[k] },
  set(o, k, v) { if (o === null || o === undefined) throw new TypeError(`Cannot set properties of ${o} (setting '${k}')`); o[k] = v; return v },
  call(o, k, ...a) {
    const f = rt.get(o, k)
    if (typeof f !== "function") throw new TypeError(`${k} is not a function`)
    return f.apply(o, a)
  },
}

class BodyRt {
  pull(other, dt) {
    const dx = rt.get(this, "x") - rt.get(other, "x")
    const dy = rt.get(this, "y") - rt.get(other, "y")
    const dz = rt.get(this, "z") - rt.get(other, "z")
    const d2 = dx * dx + dy * dy + dz * dz
    const mag = dt / (d2 * Math.sqrt(d2))
    const m2mag = rt.get(other, "mass") * mag
    rt.set(this, "vx", rt.get(this, "vx") - dx * m2mag)
    rt.set(this, "vy", rt.get(this, "vy") - dy * m2mag)
    rt.set(this, "vz", rt.get(this, "vz") - dz * m2mag)
    const m1mag = rt.get(this, "mass") * mag
    rt.set(other, "vx", rt.get(other, "vx") + dx * m1mag)
    rt.set(other, "vy", rt.get(other, "vy") + dy * m1mag)
    rt.set(other, "vz", rt.get(other, "vz") + dz * m1mag)
  }
  move(dt) {
    rt.set(this, "x", rt.get(this, "x") + dt * rt.get(this, "vx"))
    rt.set(this, "y", rt.get(this, "y") + dt * rt.get(this, "vy"))
    rt.set(this, "z", rt.get(this, "z") + dt * rt.get(this, "vz"))
  }
}

rt.fail = (o, k) => { throw new TypeError(`Cannot read properties of ${o} (reading '${k}')`) }
rt.notFn = (k) => { throw new TypeError(`${k} is not a function`) }
class BodyInline {
  pull(other, dt) {
    const dx = this.x - (other == null ? rt.fail(other, "x") : other.x)
    const dy = this.y - (other == null ? rt.fail(other, "y") : other.y)
    const dz = this.z - (other == null ? rt.fail(other, "z") : other.z)
    const d2 = dx * dx + dy * dy + dz * dz
    const mag = dt / (d2 * Math.sqrt(d2))
    const m2mag = (other == null ? rt.fail(other, "mass") : other.mass) * mag
    this.vx = this.vx - dx * m2mag
    this.vy = this.vy - dy * m2mag
    this.vz = this.vz - dz * m2mag
    const m1mag = this.mass * mag
    if (other == null) rt.fail(other, "vx"); other.vx = other.vx + dx * m1mag
    if (other == null) rt.fail(other, "vy"); other.vy = other.vy + dy * m1mag
    if (other == null) rt.fail(other, "vz"); other.vz = other.vz + dz * m1mag
  }
  move(dt) {
    this.x = this.x + dt * this.vx
    this.y = this.y + dt * this.vy
    this.z = this.z + dt * this.vz
  }
}
function advanceInline(sys, dt, n) {
  for (let k = 0; k < n; k++) {
    if (--F.n < 0) out()
    for (const [b1, b2] of sys.pairs) (b1 == null ? rt.fail(b1, "pull") : typeof b1.pull === "function" ? b1.pull(b2, dt) : rt.notFn("pull"))
    for (const b of sys.bodies) (b == null ? rt.fail(b, "move") : typeof b.move === "function" ? b.move(dt) : rt.notFn("move"))
  }
}
function advanceFuel(sys, dt, n) {
  for (let k = 0; k < n; k++) {
    if (--F.n < 0) out()
    for (const [b1, b2] of sys.pairs) b1.pull(b2, dt)
    for (const b of sys.bodies) b.move(dt)
  }
}
function advanceRt(sys, dt, n) {
  for (let k = 0; k < n; k++) {
    if (--F.n < 0) out()
    for (const [b1, b2] of rt.get(sys, "pairs")) rt.call(b1, "pull", b2, dt)
    for (const b of rt.get(sys, "bodies")) rt.call(b, "move", dt)
  }
}
function advanceRtGet(sys, dt, n) {
  for (let k = 0; k < n; k++) {
    if (--F.n < 0) out()
    for (const [b1, b2] of rt.get(sys, "pairs")) b1.pull(b2, dt)
    for (const b of rt.get(sys, "bodies")) b.move(dt)
  }
}
function advanceRtCall(sys, dt, n) {
  for (let k = 0; k < n; k++) {
    if (--F.n < 0) out()
    for (const [b1, b2] of sys.pairs) rt.call(b1, "pull", b2, dt)
    for (const b of sys.bodies) rt.call(b, "move", dt)
  }
}
function* advanceGen(sys, dt, n) {
  for (let k = 0; k < n; k++) {
    if (--F.n < 0) { yield "fuel"; F.n = 1e6 }
    for (const [b1, b2] of sys.pairs) b1.pull(b2, dt)
    for (const b of sys.bodies) b.move(dt)
  }
}
function* pullGen(a, b, dt) { a.pull(b, dt); if (F.n < -1) yield "never" }
function* moveGen(a, dt) { a.move(dt); if (F.n < -1) yield "never" }
function* advanceGenAll(sys, dt, n) {
  for (let k = 0; k < n; k++) {
    if (--F.n < 0) { yield "fuel"; F.n = 1e6 }
    for (const [b1, b2] of sys.pairs) yield* pullGen(b1, b2, dt)
    for (const b of sys.bodies) yield* moveGen(b, dt)
  }
}
function drive(g) { for (let r = g.next(); !r.done; r = g.next()) F.outs++ }

// The form of the VM, as the buffer of section 8 would carry it: a list of nodes, each a kind and its fields, with
// references to other nodes by number
const FIELDS = ["x", "y", "z", "vx", "vy", "vz", "mass"]
function toVm(sys) {
  const ids = new Map(), nodes = []
  const ref = (o) => {
    if (ids.has(o)) return ids.get(o)
    const id = nodes.length
    ids.set(o, id)
    nodes.push(null)
    if (Array.isArray(o)) nodes[id] = ["array", o.map(ref)]
    else if (o instanceof Body) nodes[id] = ["Body", FIELDS.map((f) => o[f])]
    else nodes[id] = ["System", [ref(o.bodies), ref(o.pairs)]]
    return id
  }
  ref(sys)
  return nodes
}
function fromVm(nodes) {
  const objs = nodes.map(([kind]) => (kind === "array" ? [] : kind === "Body" ? Object.create(Body.prototype) : Object.create(System.prototype)))
  nodes.forEach(([kind, v], i) => {
    const o = objs[i]
    if (kind === "array") for (const r of v) o.push(objs[r])
    else if (kind === "Body") FIELDS.forEach((f, j) => { o[f] = v[j] })
    else { o.bodies = objs[v[0]]; o.pairs = objs[v[1]] }
  })
  return objs[0]
}

let sys = new System()
console.log(sys.energy())
switch (variant) {
  case "node": sys.advance(0.01, n); break
  case "fuel": advanceFuel(sys, 0.01, n); break
  case "rt": {
    for (const b of sys.bodies) Object.setPrototypeOf(b, BodyRt.prototype)
    advanceRt(sys, 0.01, n)
    for (const b of sys.bodies) Object.setPrototypeOf(b, Body.prototype)
    break
  }
  case "rt-get": {
    for (const b of sys.bodies) Object.setPrototypeOf(b, BodyRt.prototype)
    advanceRtGet(sys, 0.01, n)
    for (const b of sys.bodies) Object.setPrototypeOf(b, Body.prototype)
    break
  }
  case "inline": {
    for (const b of sys.bodies) Object.setPrototypeOf(b, BodyInline.prototype)
    advanceInline(sys, 0.01, n)
    for (const b of sys.bodies) Object.setPrototypeOf(b, Body.prototype)
    break
  }
  case "rt-call": advanceRtCall(sys, 0.01, n); break
  case "gen": drive(advanceGen(sys, 0.01, n)); break
  case "gen-all": drive(advanceGenAll(sys, 0.01, n)); break
  case "copy": {
    let vm = toVm(sys)
    for (let i = 0; i < n; i++) {
      const s = fromVm(vm)
      advanceFuel(s, 0.01, 1)
      vm = toVm(s)
    }
    sys = fromVm(vm)
    break
  }
  default: throw new Error(`unknown variant ${variant}`)
}
console.log(sys.energy())

// nbody with classes (docs/dev/PLATFORM-PLAN-ja.md section 4.5, stage P5b): sjs/bench/nbody.safe.js with each body
// an instance of the class Body and the steps as its methods, to measure method calls in ConJS
const PI = 3.14159265358979323
const SOLAR_MASS = 4 * PI * PI
const DAYS_PER_YEAR = 365.24
const PLANETS = [
  ["4.84143144246472090e+00", "-1.16032004402742839e+00", "-1.03622044471123109e-01",
   "1.66007664274403694e-03", "7.69901118419740425e-03", "-6.90460016972063023e-05", "9.54791938424326609e-04"],
  ["8.34336671824457987e+00", "4.12479856412430479e+00", "-4.03523417114321381e-01",
   "-2.76742510726862411e-03", "4.99852801234917238e-03", "2.30417297573763929e-05", "2.85885980666130812e-04"],
  ["1.28943695621391310e+01", "-1.51111514016986312e+01", "-2.23307578892655734e-01",
   "2.96460137564761618e-03", "2.37847173959480950e-03", "-2.96589568540237556e-05", "4.36624404335156298e-05"],
  ["1.53796971148509165e+01", "-2.59193146099879641e+01", "1.79258772950371181e-01",
   "2.68067772490389322e-03", "1.62824170038242295e-03", "-9.51592254519715870e-05", "5.15138902046611451e-05"],
]
class Body {
  constructor(x, y, z, vx, vy, vz, mass) {
    this.x = x
    this.y = y
    this.z = z
    this.vx = vx
    this.vy = vy
    this.vz = vz
    this.mass = mass
  }
  kinetic() { return this.mass * (this.vx * this.vx + this.vy * this.vy + this.vz * this.vz) / 2 }
  move(dt) {
    this.x += dt * this.vx
    this.y += dt * this.vy
    this.z += dt * this.vz
  }
  pull(other, dt) {
    const dx = this.x - other.x
    const dy = this.y - other.y
    const dz = this.z - other.z
    const d2 = dx * dx + dy * dy + dz * dz
    const mag = dt / (d2 * Math.sqrt(d2))
    const m2mag = other.mass * mag
    this.vx -= dx * m2mag
    this.vy -= dy * m2mag
    this.vz -= dz * m2mag
    const m1mag = this.mass * mag
    other.vx += dx * m1mag
    other.vy += dy * m1mag
    other.vz += dz * m1mag
  }
  potential(other) {
    const dx = this.x - other.x
    const dy = this.y - other.y
    const dz = this.z - other.z
    return (this.mass * other.mass) / Math.sqrt(dx * dx + dy * dy + dz * dz)
  }
}

class System {
  constructor() {
    this.bodies = [new Body(0, 0, 0, 0, 0, 0, SOLAR_MASS)]
    for (const p of PLANETS) {
      const [x, y, z, vx, vy, vz, m] = p.map((s) => Number(s))
      this.bodies.push(new Body(x, y, z, vx * DAYS_PER_YEAR, vy * DAYS_PER_YEAR, vz * DAYS_PER_YEAR, m * SOLAR_MASS))
    }
    this.pairs = []
    for (let i = 0; i < this.bodies.length; i++)
      for (let j = i + 1; j < this.bodies.length; j++) this.pairs.push([this.bodies[i], this.bodies[j]])
    this.offsetMomentum()
  }
  offsetMomentum() {
    let px = 0, py = 0, pz = 0
    for (const b of this.bodies) {
      px -= b.vx * b.mass
      py -= b.vy * b.mass
      pz -= b.vz * b.mass
    }
    const sun = this.bodies[0]
    sun.vx = px / SOLAR_MASS
    sun.vy = py / SOLAR_MASS
    sun.vz = pz / SOLAR_MASS
  }
  energy() {
    let e = 0
    for (const [b1, b2] of this.pairs) e -= b1.potential(b2)
    for (const b of this.bodies) e += b.kinetic()
    return e
  }
  advance(dt, n) {
    for (let k = 0; k < n; k++) {
      for (const [b1, b2] of this.pairs) b1.pull(b2, dt)
      for (const b of this.bodies) b.move(dt)
    }
  }
}

const system = new System()
console.log(system.energy())
system.advance(0.01, N_ARG)
console.log(system.energy())

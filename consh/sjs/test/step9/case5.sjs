// 配列の計画の段階 A3：consh/sjs/bench/nbody.sjs（N を 100 にしたもの）。offsetMomentum、energy、advance が数の関数になる
// nbody: the orbits of the Jovian planets (bench/nbody/nbody.py), each body an array [x, y, z, vx, vy, vz, mass]
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
const X = 0, Y = 1, Z = 2, VX = 3, VY = 4, VZ = 5, M = 6

function makeBodies() {
  const bodies = [[0, 0, 0, 0, 0, 0, SOLAR_MASS]]
  for (const p of PLANETS) {
    const [x, y, z, vx, vy, vz, m] = p.map((s) => Number(s))
    bodies.push([x, y, z, vx * DAYS_PER_YEAR, vy * DAYS_PER_YEAR, vz * DAYS_PER_YEAR, m * SOLAR_MASS])
  }
  return bodies
}

function makePairs(bodies) {
  const pairs = []
  for (let i = 0; i < bodies.length; i++)
    for (let j = i + 1; j < bodies.length; j++) pairs.push([bodies[i], bodies[j]])
  return pairs
}

function offsetMomentum(bodies) {
  let px = 0, py = 0, pz = 0
  for (const b of bodies) {
    px -= b[VX] * b[M]
    py -= b[VY] * b[M]
    pz -= b[VZ] * b[M]
  }
  const sun = bodies[0]
  sun[VX] = px / SOLAR_MASS
  sun[VY] = py / SOLAR_MASS
  sun[VZ] = pz / SOLAR_MASS
}

function energy(bodies, pairs) {
  let e = 0
  for (const [b1, b2] of pairs) {
    const dx = b1[X] - b2[X]
    const dy = b1[Y] - b2[Y]
    const dz = b1[Z] - b2[Z]
    e -= (b1[M] * b2[M]) / Math.sqrt(dx * dx + dy * dy + dz * dz)
  }
  for (const b of bodies) e += b[M] * (b[VX] * b[VX] + b[VY] * b[VY] + b[VZ] * b[VZ]) / 2
  return e
}

function advance(bodies, pairs, dt, n) {
  for (let k = 0; k < n; k++) {
    for (const [b1, b2] of pairs) {
      const dx = b1[X] - b2[X]
      const dy = b1[Y] - b2[Y]
      const dz = b1[Z] - b2[Z]
      const d2 = dx * dx + dy * dy + dz * dz
      const mag = dt / (d2 * Math.sqrt(d2))
      const m2mag = b2[M] * mag
      b1[VX] -= dx * m2mag
      b1[VY] -= dy * m2mag
      b1[VZ] -= dz * m2mag
      const m1mag = b1[M] * mag
      b2[VX] += dx * m1mag
      b2[VY] += dy * m1mag
      b2[VZ] += dz * m1mag
    }
    for (const b of bodies) {
      b[X] += dt * b[VX]
      b[Y] += dt * b[VY]
      b[Z] += dt * b[VZ]
    }
  }
}

const bodies = makeBodies()
const pairs = makePairs(bodies)
offsetMomentum(bodies)
console.log(energy(bodies, pairs))
advance(bodies, pairs, 0.01, 100)
console.log(energy(bodies, pairs))

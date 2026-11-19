# (( )) の SJS で、木星型惑星の軌道を 30 万ステップ（CPython で約 1 秒）。consh/examples/nbody.sh の N を 300000 にしたもの
# bench/nbody/nbody.py と同じ計算（演算の順は sjs/bench/nbody.safe.js）。数と数の配列だけを扱うので、wasm 版の VM では WebAssembly で動く
n=300000
(( PI = 3.14159265358979323, SOLAR_MASS = 4 * PI * PI, DAYS_PER_YEAR = 365.24 ))
(( bodies = [
  [0, 0, 0, 0, 0, 0, SOLAR_MASS],
  [4.84143144246472090e+00, -1.16032004402742839e+00, -1.03622044471123109e-01,
   1.66007664274403694e-03, 7.69901118419740425e-03, -6.90460016972063023e-05, 9.54791938424326609e-04],
  [8.34336671824457987e+00, 4.12479856412430479e+00, -4.03523417114321381e-01,
   -2.76742510726862411e-03, 4.99852801234917238e-03, 2.30417297573763929e-05, 2.85885980666130812e-04],
  [1.28943695621391310e+01, -1.51111514016986312e+01, -2.23307578892655734e-01,
   2.96460137564761618e-03, 2.37847173959480950e-03, -2.96589568540237556e-05, 4.36624404335156298e-05],
  [1.53796971148509165e+01, -2.59193146099879641e+01, 1.79258772950371181e-01,
   2.68067772490389322e-03, 1.62824170038242295e-03, -9.51592254519715870e-05, 5.15138902046611451e-05],
] ))
# velocities in AU per year, masses in units of the solar mass; then the momentum of the sun
(( init = (bodies, SOLAR_MASS, DAYS_PER_YEAR) => {
  for (let i = 1; i < bodies.length; i++) {
    const b = bodies[i]
    b[3] *= DAYS_PER_YEAR; b[4] *= DAYS_PER_YEAR; b[5] *= DAYS_PER_YEAR; b[6] *= SOLAR_MASS
  }
  let px = 0, py = 0, pz = 0
  for (const b of bodies) { px -= b[3] * b[6]; py -= b[4] * b[6]; pz -= b[5] * b[6] }
  bodies[0][3] = px / SOLAR_MASS; bodies[0][4] = py / SOLAR_MASS; bodies[0][5] = pz / SOLAR_MASS
} ))
(( energy = (bodies) => {
  let e = 0
  for (let i = 0; i < bodies.length; i++) {
    const b1 = bodies[i]
    for (let j = i + 1; j < bodies.length; j++) {
      const b2 = bodies[j]
      const dx = b1[0] - b2[0], dy = b1[1] - b2[1], dz = b1[2] - b2[2]
      e -= (b1[6] * b2[6]) / Math.sqrt(dx * dx + dy * dy + dz * dz)
    }
  }
  for (const b of bodies) e += b[6] * (b[3] * b[3] + b[4] * b[4] + b[5] * b[5]) / 2
  return e
} ))
(( advance = (bodies, dt, n) => {
  for (let k = 0; k < n; k++) {
    for (let i = 0; i < bodies.length; i++) {
      const b1 = bodies[i]
      for (let j = i + 1; j < bodies.length; j++) {
        const b2 = bodies[j]
        const dx = b1[0] - b2[0], dy = b1[1] - b2[1], dz = b1[2] - b2[2]
        const d2 = dx * dx + dy * dy + dz * dz
        const mag = dt / (d2 * Math.sqrt(d2))
        const m2mag = b2[6] * mag
        b1[3] -= dx * m2mag; b1[4] -= dy * m2mag; b1[5] -= dz * m2mag
        const m1mag = b1[6] * mag
        b2[3] += dx * m1mag; b2[4] += dy * m1mag; b2[5] += dz * m1mag
      }
    }
    for (const b of bodies) { b[0] += dt * b[3]; b[1] += dt * b[4]; b[2] += dt * b[5] }
  }
} ))
(( init(bodies, SOLAR_MASS, DAYS_PER_YEAR) ))
echo $(( energy(bodies) ))
(( advance(bodies, 0.01, +$n) ))
echo $(( energy(bodies) ))

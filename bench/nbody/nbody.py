"""nbody: the orbits of the Jovian planets around the sun (the n-body
benchmark of pyperformance and of the Computer Language Benchmarks Game).

    python3 nbody.py N

Prints the energy of the system, advances it N steps of 0.01 days, and
prints the energy again. nbody.se does the same operations in the same
order, so both print the same two numbers (repr of the float)."""

import sys
from math import sqrt

PI = 3.14159265358979323
SOLAR_MASS = 4 * PI * PI
DAYS_PER_YEAR = 365.24

# x y z vx vy vz mass (velocities in AU per year, masses in solar masses)
PLANETS = [
    ("4.84143144246472090e+00", "-1.16032004402742839e+00", "-1.03622044471123109e-01",
     "1.66007664274403694e-03", "7.69901118419740425e-03", "-6.90460016972063023e-05",
     "9.54791938424326609e-04"),
    ("8.34336671824457987e+00", "4.12479856412430479e+00", "-4.03523417114321381e-01",
     "-2.76742510726862411e-03", "4.99852801234917238e-03", "2.30417297573763929e-05",
     "2.85885980666130812e-04"),
    ("1.28943695621391310e+01", "-1.51111514016986312e+01", "-2.23307578892655734e-01",
     "2.96460137564761618e-03", "2.37847173959480950e-03", "-2.96589568540237556e-05",
     "4.36624404335156298e-05"),
    ("1.53796971148509165e+01", "-2.59193146099879641e+01", "1.79258772950371181e-01",
     "2.68067772490389322e-03", "1.62824170038242295e-03", "-9.51592254519715870e-05",
     "5.15138902046611451e-05"),
]

X, Y, Z, VX, VY, VZ, M = range(7)


def make_bodies():
    """The sun and the planets, each a list [x, y, z, vx, vy, vz, mass]."""
    bodies = [[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, SOLAR_MASS]]
    for p in PLANETS:
        x, y, z, vx, vy, vz, m = (float(s) for s in p)
        bodies.append([x, y, z, vx * DAYS_PER_YEAR, vy * DAYS_PER_YEAR, vz * DAYS_PER_YEAR, m * SOLAR_MASS])
    return bodies


def make_pairs(bodies):
    pairs = []
    for i in range(len(bodies)):
        for j in range(i + 1, len(bodies)):
            pairs.append((bodies[i], bodies[j]))
    return pairs


def offset_momentum(bodies):
    """Give the sun the momentum that makes the total momentum zero."""
    px = py = pz = 0.0
    for b in bodies:
        px -= b[VX] * b[M]
        py -= b[VY] * b[M]
        pz -= b[VZ] * b[M]
    sun = bodies[0]
    sun[VX] = px / SOLAR_MASS
    sun[VY] = py / SOLAR_MASS
    sun[VZ] = pz / SOLAR_MASS


def energy(bodies, pairs):
    e = 0.0
    for b1, b2 in pairs:
        dx = b1[X] - b2[X]
        dy = b1[Y] - b2[Y]
        dz = b1[Z] - b2[Z]
        e -= (b1[M] * b2[M]) / sqrt(dx * dx + dy * dy + dz * dz)
    for b in bodies:
        e += b[M] * (b[VX] * b[VX] + b[VY] * b[VY] + b[VZ] * b[VZ]) / 2.0
    return e


def advance(bodies, pairs, dt, n):
    for _ in range(n):
        for b1, b2 in pairs:
            dx = b1[X] - b2[X]
            dy = b1[Y] - b2[Y]
            dz = b1[Z] - b2[Z]
            d2 = dx * dx + dy * dy + dz * dz
            mag = dt / (d2 * sqrt(d2))
            m2mag = b2[M] * mag
            b1[VX] -= dx * m2mag
            b1[VY] -= dy * m2mag
            b1[VZ] -= dz * m2mag
            m1mag = b1[M] * mag
            b2[VX] += dx * m1mag
            b2[VY] += dy * m1mag
            b2[VZ] += dz * m1mag
        for b in bodies:
            b[X] += dt * b[VX]
            b[Y] += dt * b[VY]
            b[Z] += dt * b[VZ]


def main():
    n = int(sys.argv[1])
    bodies = make_bodies()
    pairs = make_pairs(bodies)
    offset_momentum(bodies)
    print(repr(energy(bodies, pairs)))
    advance(bodies, pairs, 0.01, n)
    print(repr(energy(bodies, pairs)))


if __name__ == "__main__":
    main()

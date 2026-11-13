// -0：JavaScript と同じ（consh/SJS-WASM-PLAN-ja.md の 11）。1 を -0 で割ると -Infinity になることで見る。Math.round(0.49999999999999994) と 5 % Infinity も
const inv = (x) => 1 / x
const z = 0
const nz = -z
console.log("div", 1 / -0, 1 / 0, -1 / -0, 0 / -0, -5 / nz, inv(nz))
console.log("neg", inv(-z), inv(-nz), inv(-(1 - 1)))
console.log("mul", inv(-0 * 5), inv(0 * -5), inv(nz * nz))
console.log("add", inv(nz + 0), inv(nz - 0), inv(nz + nz), inv(0 - 0))
console.log("trunc", inv(Math.trunc(-0.5)), inv(Math.trunc(0.5)), inv(Math.trunc(nz)), Math.trunc(-1.5))
console.log("ceil", inv(Math.ceil(-0.5)), inv(Math.ceil(nz)), Math.ceil(-1.5), Math.ceil(1.2))
console.log("floor", inv(Math.floor(nz)), inv(Math.floor(0.5)), Math.floor(-0.5))
console.log("round", inv(Math.round(-0.5)), inv(Math.round(-0.2)), inv(Math.round(nz)), inv(Math.round(0.2)))
console.log("round2", Math.round(0.49999999999999994), Math.round(2.5), Math.round(-2.5), Math.round(-0.6), Math.round(1e300), Math.round(-4503599627370495.5))
console.log("abs", inv(Math.abs(nz)), Math.abs(-3), inv(Math.abs(0)))
console.log("sign", inv(Math.sign(nz)), inv(Math.sign(0)), Math.sign(-3))
console.log("max", inv(Math.max(nz, 0)), inv(Math.max(0, nz)), inv(Math.max(nz, nz)), inv(Math.max(nz)), Math.max(), Math.max(1, NaN, 2))
console.log("min", inv(Math.min(nz, 0)), inv(Math.min(0, nz)), inv(Math.min(0, 0)), Math.min(), Math.min(3, 1, 2))
console.log("mod", inv(-1 % 1), inv(nz % 5), inv(0 % -5), inv(-4 % 2), 5 % Infinity, -5 % -Infinity, Infinity % 5, 5 % 0, nz % 0, -7 % 3)
console.log("str", String(nz), `${nz}`, JSON.stringify([nz]), nz === 0, [1, 2][nz])
console.log("number", inv(Number("-0")), inv(Number("-0.0")), inv(parseInt("-0")))
let a = 0
a = -a
a /= 0
console.log("assign", a)

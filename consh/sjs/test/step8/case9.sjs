// ビット演算（& | ^ ~ << >> >>>）：JavaScript と同じ 32 ビット（高速版は高速版 ConMa のプリミティブ、参照版は算術。docs/BITWISE-PLAN-ja.md）
const values = [0, 1, -1, 5, -12345, 53256, 2147483647, 2147483648, -2147483649, 4294967295, 4294967297, 1.9, -1.9, -0.5, 1e10, -1e10, 1e300, NaN, Infinity, -Infinity]
const counts = [0, 1, 31, 32, 33, -1, 1.5, NaN]
const rows = []
for (const a of values) {
  rows.push([a & 53256, a | 3, a ^ 53256, ~a, a & -1, a | 0])
  for (const n of counts) rows.push([a << n, a >> n, a >>> n])
}
console.log(JSON.stringify(rows))
let x = 12345
for (let i = 0; i < 100; i++) x = ((x * 31 + (x >>> 7)) ^ (i << 3)) & 1048575
console.log(x)
try { console.log("a" & 1) } catch (e) { console.log(`${e}`) }

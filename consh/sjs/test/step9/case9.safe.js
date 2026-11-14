// 段階 F2（consh/SJS-WASM-ARRAY-PLAN-ja.md の 18.3、20）：添字が定数の読み書きを、長さの確かめと f64.load / f64.store で
// その場に書く。範囲の中、長さちょうど（読めば undefined、書けば配列がのびる。wasm 版は trap にして ConMa 版で動かす）、-0、
// 数の定数の添字、小数の添字、大きな添字、+= と ++、配列の配列の要素（式の配列を $__o に入れる）、値の中の読み書き、
// 要素が足りない分解、for...of の要素（確かめずに読む）と length
const K = 2
function get3(a) { return a[0] + a[1] * 10 + a[K] * 100 }
function last(a) { return a[3] }
function negZero(a) { return a[-0] }
function frac(a) { return a[1.5] }
function far(a) { return a[300000000] }
function bump(a) {
  a[0] += 5
  a[1]++
  a[K] -= 0.5
}
function grow(a) { a[3] = 7 }
function nested(m) {
  m[1][0] = m[2][1] * 2 + m[0][2]
  m[2][2] += m[1][0]
  return m[1][0] + m[2][2]
}
function pairs(ps) {
  let s = 0
  for (const [x, y] of ps) s += x * y
  return s
}
function second(ps) {
  let r = 0
  for (const [x, y] of ps) r = y
  return r
}
function lens(m) {
  let s = 0
  for (const row of m) s += row.length * 100 + row[0]
  return s + m.length
}
const a = [1, 2, 3]
console.log(get3(a), last(a), negZero(a), frac(a), far(a))
bump(a)
console.log(a.join(" "))
const b = [1, 2, 3]
grow(b)
console.log(b.length, b.join(" "))
const m = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
console.log(nested(m), m[1].join(" "), m[2].join(" "))
console.log(pairs([[1, 2], [3, 4]]), second([[1, 2], [3, 4]]), second([[1, 2], [3]]), lens(m), lens([[5], [6, 7]]))

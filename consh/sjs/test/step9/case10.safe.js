// 段階 F4（consh/docs/dev/SJS-WASM-ARRAY-PLAN-ja.md の 22）：添字が変数の読み書きも、整数か、0 以上か、長さより小さいかの確かめと
// f64.load / f64.store でその場に書く（$__get と $__set を呼ばない）。範囲の中、長さちょうど（読めば undefined、書けば
// 配列がのびる。wasm 版は trap にして ConMa 版で動かす）、負の数、小数、NaN、-0、式の添字、添字の中の読み書き、
// 式の配列（配列の配列）、+= と ++（同じ場所を一度だけ確かめる）、値の中で同じ要素に書く関数、数の関数の値の添字
function sum(a) {
  let s = 0
  for (let i = 0; i < a.length; i++) s += a[i]
  return s
}
function at(a, i) { return a[i] }
function put(a, i, v) { a[i] = v }
function prefix(a) {
  for (let i = 1; i < a.length; i++) a[i] = a[i - 1] + a[i]
}
function pick(a, b) {
  let s = 0
  for (let i = 0; i < b.length; i++) s = s * 10 + a[b[i]]
  return s
}
function transpose(m) {
  for (let i = 0; i < m.length; i++) {
    for (let j = i + 1; j < m.length; j++) {
      const t = m[i][j]
      m[i][j] = m[j][i]
      m[j][i] = t
    }
  }
}
function addDiag(m, v) {
  for (let i = 0; i < m.length; i++) {
    m[i][i] += v
    m[i][m.length - 1 - i]++
  }
}
function setTo100(a, k) { a[k] = 100; return 5 }
function assignAfter(a, k) { a[k] = setTo100(a, k) }
function addAfter(a, k) { a[k] += setTo100(a, k) }
function twice(i) { return i * 2 }
function viaCall(a, i) { a[twice(i)] *= 3; return a[twice(i) + 1] }
const a = [1, 2, 3, 4]
console.log(sum(a), at(a, 2), at(a, 4), at(a, -1), at(a, 1.5), at(a, NaN), at(a, -0))
put(a, 1, 20)
put(a, 4, 5)
console.log(a.length, a.join(" "))
const p = [1, 2, 3, 4, 5]
prefix(p)
console.log(p.join(" "), pick([7, 8, 9], [2, 0, 1, 1]))
const m = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
transpose(m)
console.log(m.map(r => r.join(" ")).join(" / "))
addDiag(m, 10)
console.log(m.map(r => r.join(" ")).join(" / "))
const c = [1, 2, 3]
assignAfter(c, 1)
const d = [1, 2, 3]
addAfter(d, 1)
console.log(c.join(" "), d.join(" "))
const e = [1, 2, 3, 4, 5, 6]
console.log(viaCall(e, 1), e.join(" "), viaCall(e, 2), e.join(" "))

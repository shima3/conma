// 配列の計画（consh/SJS-WASM-ARRAY-PLAN-ja.md）の段階 A3：配列を使う関数も数の関数になる。引数の配列は wasm のメモリに写され、
// 書き換えた数は書き戻される。配列は JSON.stringify で出す（console.log の配列の書き方は Node.js と違う）
const X = 0, Y = 1
function sum(a) { let s = 0; for (const v of a) s += v; return s }
function sumIdx(a) { let s = 0; for (let i = 0; i < a.length; i++) s += a[i]; return s }
function scale(a, k) { for (let i = 0; i < a.length; i++) a[i] *= k }
function inc(a) { a[0]++; a[1] += 2; a[2] -= 1 }
function dot(ps) { let s = 0; for (const [u, v] of ps) s += u[X] * v[Y]; return s }
function first(m) { const row = m[1]; return row[0] + m[0][1] }
function total(m) { let s = 0; for (const row of m) s += sum(row); return s }
function swap(a) { const t = a[0]; a[0] = a[1]; a[1] = t }
function bumpPairs(ps) { for (const [u, v] of ps) { u[0] += 1; v[0] += 1 } }
function lenOnly(a) { return a.length }
function count(a) { let n = 0; for (const v of a) n += 1; return n }
function maxOf(a) { let m = -Infinity; for (const v of a) { if (v > m) m = v } return m }
function firstNeg(a) { for (let i = 0; i < a.length; i++) { if (a[i] < 0) return i } return -1 }
// 長さより後に書く：wasm 版は trap になり、ConMa 版が配列を延ばす
function grow(a) { a[a.length] = 9 }
// 数の関数にならない：push、配列を作る。引数をそのまま返す関数は、数を受け取って数を返す数の関数になり、配列を渡すと
// 形が合わないので ConMa 版で動く
function pushes(a) { a.push(1); return a.length }
function makes(n) { const a = [n]; return a[0] }
function returnsArray(a) { return a }

const a = [1, 2, 3]
console.log(sum(a), sumIdx(a), lenOnly(a), count([]), maxOf([3, -1, 7, 2]), maxOf([]), firstNeg([4, 5, -6]), firstNeg([1]))
scale(a, 2)
console.log(JSON.stringify(a))
inc(a)
console.log(JSON.stringify(a))
const p = [1, 2], q = [3, 4]
console.log(dot([[p, q], [q, p], [p, p]]))
const m = [[1, 2], [3, 4], [5, 6]]
console.log(first(m), total(m))
swap(p)
console.log(JSON.stringify(p))
// 同じ配列を 2 回入れた組：u と v は同じ配列なので、2 を足す
const shared = [10, 20]
bumpPairs([[shared, shared], [p, q]])
console.log(JSON.stringify(shared), JSON.stringify(p), JSON.stringify(q))
grow(a)
console.log(JSON.stringify(a))
// 数でない要素の配列、深さの違う配列は、写さずに ConMa 版で動く
console.log(lenOnly(["x", "y"]), lenOnly([[1], [2], [3]]), count([[1], [2]]))
console.log(pushes([1]), makes(3), JSON.stringify(returnsArray([4])))
// -0 の要素も、そのまま書き戻す
const z = [-0, 1]
scale(z, 1)
console.log(1 / z[0])

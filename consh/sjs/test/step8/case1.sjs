// in 演算子（配列の穴、メソッドの名前、オブジェクトでない値の TypeError）
const o = { a: 1, b: undefined }
const arr = [1, , 3]
const f = (x) => x
function* g() {}
console.log("a" in o, "b" in o, "c" in o, 0 in arr, 1 in arr, 2 in arr, 3 in arr, "2" in arr, "length" in arr, "push" in arr)
console.log("length" in f, "name" in f, "next" in g(), "then" in Promise.resolve(1), !("x" in o), "a" in o === true)
const k = "a"; let n = 0; for (const key of ["a", "z"]) if (key in o) n++
console.log(n, k in o ? "yes" : "no")
for (const bad of [5, "str", null, undefined, true]) { try { "x" in bad } catch (e) { console.log(e.name, e.message) } }

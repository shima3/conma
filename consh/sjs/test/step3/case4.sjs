// 受け手のない実行時のエラー（関数の中）
const f = (o) => o.a.b
console.log(f({ a: { b: 1 } }))
console.log(f({}))

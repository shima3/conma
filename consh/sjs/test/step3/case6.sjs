// 段階 B9：呼び出しから戻った後の、同じ文の式のエラー
const f = (x) => x * 2
const v = { n: null }
console.log(f(3), v.n.size)

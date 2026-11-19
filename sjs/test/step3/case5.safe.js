// 段階 B9：複数の行にまたがる式の中のエラー（位置は、失敗したメンバーの名前）
const o = { a: 1 }
const total = 1 +
  o.a +
  o.missing.deep
console.log(total)

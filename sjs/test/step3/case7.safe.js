// 段階 B9：入れ子の関数の中のエラー（いちばん内側の位置）
function outer(k) {
  const inner = (m) => {
    const t = m.list
    return t.length
  }
  return inner(k)
}
console.log(outer({ list: [1, 2] }))
console.log(outer({}))

// 段階 F1（consh/SJS-WASM-ARRAY-PLAN-ja.md の 19）：燃料を関数ごとに覚え、切れたら 8 倍にしてやり直す。配列を写す関数が、
// 燃料が切れてやり直しても（2^20、2^23、2^26 で切れる）、一度だけ動かしたのと同じ結果を書き戻すこと。覚えた大きな燃料で、
// 短い呼び出しと、配列の長さが変わった呼び出しも同じ結果になること
function bump(a, n) {
  for (let k = 0; k < n; k++)
    for (let i = 0; i < a.length; i++) a[i] = a[i] + (i + 1)
}
function total(a, n) {
  let s = 0
  for (let k = 0; k < n; k++)
    for (let i = 0; i < a.length; i++) s += a[i] % 3
  return s
}
const a = [0, 0, 0, 0]
bump(a, 3000000)
console.log(a.join(" "))
bump(a, 1)
console.log(a.join(" "))
a.push(10)
bump(a, 2)
console.log(a.join(" "))
console.log(total(a, 2000000), total(a, 1), total([1, 2], 3))

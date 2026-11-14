// 深い再帰：wasm の最初の実行（最適化の前）はスタックの枠が大きく、深さ 100 万の再帰は trap になる。そのときは ConMa 版で
// 動かす（ConMa 版の中の呼び出しは、wasm 版を試し直さない。consh/SJS-WASM-PLAN-ja.md の段階 3）。Node.js では
// RangeError になるので、ログは手で書いた
function deep(n) { return n === 0 ? 0 : 1 + deep(n - 1) }
console.log(deep(1000000))

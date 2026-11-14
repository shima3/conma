// 数の関数の wasm 版の燃料（consh/SJS-WASM-PLAN-ja.md の段階 3）：燃料 2^20 で始め、使い切るたびに 8 倍（上限 2^30 まで）に
// してやり直す（consh/SJS-WASM-ARRAY-PLAN-ja.md の 19。前は 2^16 から 2 倍ずつ）。countdown(30000000) は 2^25 の燃料が要る
// （2^20 と 2^23 で切れ、2^26 で終わる）。ConMa 版では遅すぎるので、wasm 版の VM だけで流す
function countdown(n) {
  let acc = 0
  while (n !== 0) {
    n -= 1
    acc += 1
  }
  return acc
}
function sumTo(n) {
  let s = 0
  for (let i = 1; i <= n; i++) s += i % 7
  return s
}
function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2) }
console.log(countdown(65535), countdown(65536), countdown(30000000))
console.log(sumTo(10000000), fib(30))
let total = 0
for (let i = 0; i < 100; i++) total += countdown(100000)
console.log(total)

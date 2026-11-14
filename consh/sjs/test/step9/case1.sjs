// 数の関数の wasm 版（consh/SJS-WASM-PLAN-ja.md の段階 2）：境の値。wasm 版の VM（CONMA_VM=wasm）では数の関数が wasm で、
// ほかの VM では ConMa 版で動き、どちらも Node.js と同じ結果になる。-0 は、1 をそれで割った -Infinity で見る
const K = 10
const inv = (x) => 1 / x
function add(a, b) { return a + b }
function sub(a, b) { return a - b }
function mul(a, b) { return a * b }
function div(a, b) { return a / b }
function mod(a, b) { return a % b }
function neg(a) { return -a }
function pos(a) { return +a }
function band(a, b) { return a & b }
function bor(a, b) { return a | b }
function bxor(a, b) { return a ^ b }
function shl(a, b) { return a << b }
function shr(a, b) { return a >> b }
function ushr(a, b) { return a >>> b }
function bnot(a) { return ~a }
function cmp(a, b) {
  let r = 0
  if (a < b) r += 1
  if (a <= b) r += 2
  if (a > b) r += 4
  if (a >= b) r += 8
  if (a === b) r += 16
  if (a !== b) r += 32
  return r
}
function truthy(x) { return x ? 1 : 0 }
function not(x) { return !x ? 1 : 0 }
function logic(a, b) { return (a < b && b < 10) || a === 0 ? 1 : 0 }
function floor(x) { return Math.floor(x) }
function ceil(x) { return Math.ceil(x) }
function trunc(x) { return Math.trunc(x) }
function abs(x) { return Math.abs(x) }
function sqrt(x) { return Math.sqrt(x) }
function round(x) { return Math.round(x) }
function sign(x) { return Math.sign(x) }
function max2(a, b) { return Math.max(a, b) }
function min3(a, b, c) { return Math.min(a, b, c) }
function max0() { return Math.max() }
function pi() { return Math.PI * K }
function sum(n) {
  let s = 0
  for (let i = 1; i <= n; i++) {
    if (i % 3 === 0) continue
    if (i > 50) break
    s += i
  }
  return s
}
function bits(n) {
  let c = 0
  do {
    c++
    n = n >>> 1
  } while (n > 0)
  return c
}
function fact(n) { return n <= 1 ? 1 : n * fact(n - 1) }
function deep(n) { return n === 0 ? 0 : 1 + deep(n - 1) }
function countdown(n) {
  let acc = 0
  while (n !== 0) {
    n -= 1
    acc += 1
  }
  return acc
}
function compound(a) {
  let x = a
  x += 1
  x -= 2
  x *= 3
  x /= 4
  x %= 5
  x &= 7
  x |= 8
  x ^= 3
  x <<= 2
  x >>= 1
  x >>>= 0
  return x
}
function nan() { return NaN }
function ninf() { return -Infinity }
function id(x) { return x }

console.log("add", add(1, 2), add(0.1, 0.2), add(1e308, 1e308), inv(add(-0, -0)), inv(add(-0, 0)))
console.log("sub mul", sub(1, 3), inv(sub(-0, 0)), mul(1e200, 1e200), inv(mul(-1, 0)), mul(3, -7))
console.log("div", div(1, 0), div(-1, 0), div(1, -0), div(0, 0), inv(div(1, -Infinity)), div(7, 2))
console.log("mod", mod(7, 3), mod(-7, 3), inv(mod(-1, 1)), inv(mod(-0, 5)), mod(5, Infinity), mod(-5, -Infinity), mod(Infinity, 5), mod(5, 0), mod(NaN, 1), mod(1, NaN), mod(5.5, 2))
console.log("neg pos", inv(neg(0)), inv(neg(-0)), neg(NaN), pos(-3))
console.log("bits", band(6, 3), bor(6, 3), bxor(6, 3), shl(1, 31), shl(1, 32), shr(-8, 1), ushr(-8, 1), ushr(-1, 0), bnot(0), bnot(-1.9))
console.log("bits2", band(4294967297, 3), bor(2147483648, 0), bxor(1e300, 1), band(NaN, 1), shl(1, -1), shr(Infinity, 1), ushr(-0.5, 0))
console.log("cmp", cmp(1, 2), cmp(2, 1), cmp(1, 1), cmp(0, -0), cmp(NaN, 1), cmp(1, NaN), cmp(-Infinity, Infinity))
console.log("truthy", truthy(0), truthy(-0), truthy(NaN), truthy(1), truthy(-0.5), not(0), not(2), logic(1, 2), logic(1, 20), logic(0, 0), logic(3, 1))
console.log("floor ceil trunc", floor(-0.5), inv(floor(-0)), inv(ceil(-0.5)), ceil(1.2), inv(trunc(-0.5)), trunc(-1.5), trunc(1e300))
console.log("abs sqrt", abs(-3), inv(abs(-0)), sqrt(2), sqrt(-1), inv(sqrt(-0)), sqrt(Infinity))
console.log("round", round(2.5), round(-2.5), round(0.49999999999999994), inv(round(-0.2)), inv(round(-0)), round(-0.6), round(1e300), round(NaN))
console.log("sign", sign(-3), sign(2), inv(sign(-0)), inv(sign(0)), sign(NaN))
console.log("max min", max2(1, 2), inv(max2(-0, 0)), inv(max2(0, -0)), max2(1, NaN), min3(3, 1, 2), inv(min3(0, -0, 0)), min3(1, NaN, 0), max0(), pi())
console.log("loops", sum(10), sum(100), bits(0), bits(1), bits(255), bits(-1), countdown(1000))
console.log("calls", fact(10), fact(170), fact(171), deep(5000))
console.log("compound", compound(10), compound(-7.5), compound(1e10))
console.log("consts", nan(), ninf())
// 数でない引数、足りない引数、余った引数：ConMa 版で動く（数の関数は数だけで作られるが、引数は SJS の値なら何でもよい）
console.log("not numbers", id("abc"), id(true), id(), truthy(""), truthy("a"), truthy(null), add(1, 2, 3))

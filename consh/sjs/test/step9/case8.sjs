// % は正確な剰余（C の fmod。REVIEW-ja.md の 2026-11-14（2）の「前からの違い」、docs/ROADMAP.md の Step 44）。
// ConMa 版は __Number_remainder__、数の関数の wasm 版は $__mod（倍にして引き、半分にする）で、どちらも JavaScript と同じ
function m(a, b) { return a % b }
const r = (x) => (1 / x === -Infinity ? "-0" : String(x))
console.log(r(m(5.5, 1.1)), r(m(1e17, 3)), r(m(1e300, 7)), r(m(0.3, 0.1)), r(m(1e-320, 3e-321)), r(m(5e-324, 1e-320)))
console.log(r(m(-7, 3)), r(m(7, -3)), r(m(-0, 5)), r(m(-4, 2)), r(m(4, -2)), r(m(1e308, 1e-308)), r(m(-1e308, 3)))
console.log(r(m(Infinity, 5)), r(m(5, Infinity)), r(m(-5, -Infinity)), r(m(NaN, 2)), r(m(2, NaN)), r(m(5, 0)), r(m(5, -0)))
let x = 1e17
x %= 7
console.log(r(x), r(17.5 % 4), r(-17.5 % 4))

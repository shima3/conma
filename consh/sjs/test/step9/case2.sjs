// 数の関数にならない関数（consh/SJS-WASM-PLAN-ja.md の 3.2）：どの VM でも ConMa 版で動く
let counter = 0
const half = (x) => x / 2
function str(a) { return `${a}` }
function log(a) { console.log("log", a); return a }
function closure(a) { const f = () => a; return f() }
function noReturn(a) { if (a) return 1 }
function writesOuter(a) { counter = counter + a; return counter }
function callsArrow(a) { return half(a) }
function usesArray(a) { const xs = [a, a]; return xs[0] + xs[1] }
function power(a) { return a ** 2 }
function boolVar(a) { const b = a > 1; return b ? 1 : 0 }
function reassigned(a) { return a + 1 }
reassigned = (a) => a + 2
console.log(str(1), log(2), closure(3), noReturn(0), noReturn(1), writesOuter(4), writesOuter(5), callsArrow(6), usesArray(7), power(8), boolVar(9), reassigned(10))

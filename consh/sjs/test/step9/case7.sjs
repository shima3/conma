// let と const はブロックの中だけで有効（REVIEW-ja.md の 2026-11-14（2）の 2）。ブロック、for、for...of を出た後の
// 同じ名前は、外の変数を読む。外になければ ReferenceError（数の関数にしない）。兄弟のブロックの同じ名前は別の変数
let t = 100
let k = -1
function f(a) { { let t = a } return t }
function g(a) { for (let i = 0; i < a; i++) { let k = i } return k }
function h(a) { let s = 0; { const x = a * 2; s += x } { const x = a * 3; s += x } return s }
function sum(n) { let s = 0; for (let i = 0; i < n; i++) s += i; for (let i = 0; i < n; i++) s += i; return s }
function nest(a) { let r = 0; { let u = a; { let v = u + 1; r = v } } return r }
function ifs(a) { if (a > 0) { let w = a; return w } else { let w = -a; return w } }
function arr(xs) { let s = 0; for (const x of xs) s += x; for (const x of xs) s += x * 10; return s }
console.log(f(7), g(3), h(1), sum(4), nest(5), ifs(-2), arr([1, 2]))
function late(a) { for (let i = 0; i < a; i++) {} return i }
try { console.log(late(3)) } catch (e) { console.log(e.name) }

// 配列の計画（consh/SJS-WASM-ARRAY-PLAN-ja.md）の段階 A0：一番外の数の定数の式と、値を返さない関数も、数の関数になる
const PI = 3.14159265358979323
const SOLAR_MASS = 4 * PI * PI
const DAYS = 365.24
const NEG = -2
const MIXED = (SOLAR_MASS - 1) / DAYS % 3
const TWICE = +NEG * -NEG
function mass(m) { return m * SOLAR_MASS }
function neg(x) { return x * NEG }
function mixed() { return MIXED + TWICE }
// 値を返さない関数：値のない return か、最後まで来る
function noop(x) { if (x > 0) return; x += 1 }
function count(n) { let c = 0; for (let i = 0; i < n; i++) c += 1 }
// 値を返さない数の関数を、文として呼ぶ
function useVoid(n) { noop(n); count(n); return n * 2 }
// 値のある return とない return が混ざる関数、値を返さない関数の値を使う関数は、数の関数にならない
function mixedReturn(x) { if (x) return 1; return }
function voidValue(x) { const v = noop(x); return v === undefined ? 1 : 0 }
console.log(mass(2), neg(3), mixed())
console.log(noop(1), noop(-1), count(5), useVoid(4), mixedReturn(0), mixedReturn(1), voidValue(1))
// 呼び出しの後の const は、数の定数にならない（その名前を読む関数は数の関数にならない）
const LATE = 5
function late(x) { return x + LATE }
console.log(late(1))

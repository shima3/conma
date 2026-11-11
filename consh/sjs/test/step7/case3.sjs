// 同じブロックの let と const の名前の重なり（関数の宣言どうしと、引数と関数の宣言は重なってよい）
function f(x) { function x() {} return typeof x }
function g() {} function g() { return 2 }
const x = 1; { let x = 2; console.log(x) } console.log(x, f(1), g())
for (const x of [1]) { let x = 2; console.log(x) }

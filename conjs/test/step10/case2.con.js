// ジェネレータ、反復、例外、switch（JavaScript のプログラムでもある。../compare.sh）
function* range(n) { for (let i = 0; i < n; i++) yield i; }
console.log([...range(5)]);
let total = 0;
for (const x of range(4)) total += x;
console.log("total:", total);
function check(v) {
  try {
    if (typeof v !== "number") throw "not a number";
    return v * 2;
  } catch (e) {
    return `caught: ${e}`;
  } finally {
    console.log("finally for", v);
  }
}
console.log(check(21), check("x"));
for (const k of ["a", "b", "z"]) {
  switch (k) {
    case "a": console.log("is a"); break;
    case "b": console.log("is b"); break;
    default: console.log("other", k);
  }
}
const m = new_map();
function new_map() { const d = {}; d["k"] = 1; return d; }
console.log("k" in m, m.k ?? "none", m.q ?? "none");

async function where() { try { null.x } catch (e) { const at = e.stack.split("\n")[1].split(":"); return `${at[at.length - 2]}:${at[at.length - 1].replace(")", "")}` } }
async function inside() { return typeof rt }
// 1 行目の宣言も、ホストのスタックの行と桁はこのファイルのもの。本体から、ランタイムの rt は見えない。
// セミコロンのない文の後の宣言も、async 関数になる（M2 のレビュー、shima3/conma#92）
const x = 1
async function next(a) { return a + 1 }
let n = 0
n = n + 1
export async function later() { return n2() }
console.log(where(), inside(), next(x))

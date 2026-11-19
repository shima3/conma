// ある処理がジェネレータの本体の中で待っている間に、ほかの処理がその next() を呼ぶと TypeError
function* g() {
  yield host.wait("test.later", 2, "first")[1];
  yield "second";
}
const it = g();
const a = spawn(() => it.next().value);
const b = spawn(() => {
  host.wait("test.later", 1);
  try { it.next(); } catch (e) { console.log("b:", String(e)); }
});
join(b);
console.log("a:", join(a));
console.log("then:", it.next().value);

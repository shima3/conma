// join は、処理の例外を投げ直す（位置は、処理の中で投げた所のまま）
const t = spawn(() => {
  host.wait("test.later", 1);
  null.x;
});
try {
  join(t);
} catch (e) {
  console.log("caught", String(e));
}
// 何度 join しても同じ例外。受け手がなければ、投げた所の位置で終える
try { join(t); } catch (e) { console.log("again", String(e)); }
const u = spawn(() => { throw new_error(); });
function new_error() { return "plain value"; }
try { join(u); } catch (e) { console.log("value", e); }
join(t);
console.log("not reached");

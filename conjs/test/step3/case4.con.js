// try の受け手は、処理ごとに分かれる
const a = spawn(() => {
  try {
    host.wait("test.later", 2, "a");
    console.log("a: no exception here");
  } catch (e) {
    console.log("a: wrongly caught", e);
  }
});
const b = spawn(() => {
  host.wait("test.later", 1, "b");
  try { throw "b's error"; } catch (e) { console.log("b: caught", e); }
  throw "b uncaught";
});
join(a);
try { join(b); } catch (e) { console.log("main: join b threw", e); }
// 投げた例外が通る finally の中で待ち、その間にほかの処理が throw と catch をしても、位置は最初に投げた所のまま
const c = spawn(() => {
  try {
    undefined.first;
  } finally {
    host.wait("test.later", 2, "c");
  }
});
const d = spawn(() => {
  host.wait("test.later", 1, "d");
  try { null.other; } catch (e) { console.log("d: caught", String(e)); }
});
join(d);
join(c);

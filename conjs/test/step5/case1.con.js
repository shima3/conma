// 行き詰まり：動かせる処理も待っている呼び出しもなく、join で待つ処理だけが残る
let a, b;
a = spawn(() => { host.wait("test.later", 0); return join(b); });
b = spawn(() => { host.wait("test.later", 0); return join(a); });
console.log("main does not join");

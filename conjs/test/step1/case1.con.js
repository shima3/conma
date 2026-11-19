// 動かす順：spawn は列の最後に入り、呼んだ側は次に待つまで動き続ける
const log = [];
const a = spawn(() => { console.log("A start"); const r = host.wait("test.later", 2, "a"); console.log("A got", r); return "A"; });
const b = spawn(() => { console.log("B start"); const r = host.wait("test.later", 1, "b"); console.log("B got", r); return "B"; });
console.log("main before join", typeof a, a);
console.log("join a:", join(a));
console.log("join b:", join(b));
console.log("main end");

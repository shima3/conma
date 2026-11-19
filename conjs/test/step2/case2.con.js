// 待たない長いループは、その間ほかの処理を止める（JavaScript と同じ）
let done = false;
const watcher = spawn(() => { console.log("watcher sees done =", done); });
let s = 0;
for (let i = 0; i < 20000; i++) s += i;
done = true;
console.log("loop finished", s);
join(watcher);
// 切り替わるのは待つ呼び出しの所だけ：列は、待つときにだけ進む
const order = [];
const t1 = spawn(() => { order.push("t1-a"); host.wait("test.later", 0); order.push("t1-b"); });
const t2 = spawn(() => { order.push("t2-a"); host.wait("test.later", 0); order.push("t2-b"); });
order.push("main");
join(t1); join(t2);
console.log(order.join(" "));

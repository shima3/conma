// Rust の API だけのこと（test/host-api.sh が vm/examples/host_demo.rs で動かす）：待つ関数を host.call で呼ぶとエラー。
// Done を別のスレッドで捨てても、エラーで終わる。待つ関数を、いくつもの処理から同時に呼ぶ
try { host.call("demo.delay", 1, "x"); } catch (e) { console.log(String(e)); }
const tasks = [];
for (let i = 0; i < 5; i++) tasks.push(spawn(() => host.wait("demo.delay", 100 * (5 - i), `t${i}`)));
console.log(tasks.map((t) => join(t)).join(" "));
console.log(host.available("demo"), host.available("time"), host.available("none"));

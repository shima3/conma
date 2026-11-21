// ホストの関数を足したプログラムで動かす見本（docs/HOST-ja.md、段階 P7）。ホストの関数 demo.* は、Rust の
// vm/examples/host_demo.rs と、JavaScript の conjs/examples/host_demo.host.js（wasm 版の VM の CONMA_HOST）に、同じものがある
if (!host.available("demo")) {
  console.log("this VM has no demo functions (see docs/HOST-ja.md)");
} else {
  console.log(host.call("demo.add", 1, 2, 3.5));
  // 待つ関数：遅いほうを別の処理で待つ間に、速いほうが先に終わる
  const slow = spawn(() => host.wait("demo.delay", 300, "slow"));
  console.log(host.wait("demo.delay", 50, "fast"));
  console.log(join(slow));
  // オブジェクトと真偽：渡すときは自分のプロパティだけ、返った Dict はオブジェクトになる
  const info = host.call("demo.info", "abc", ["x", "y"]);
  console.log(info, info.short === true);
  // エラー
  for (const f of [() => host.call("demo.add", "x"), () => host.wait("demo.delay", -1, "x"),
                   () => host.wait("demo.drop"), () => host.call("demo.none")]) {
    try { f(); } catch (e) { console.log(String(e)); }
  }
}

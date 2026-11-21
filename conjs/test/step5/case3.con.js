// ホストの呼び出し：値の行き来と、ホストのエラー
console.log(host.available("test"), host.available("no-such-feature"));
console.log(host.call("test.add", 1, 2, 3));
console.log(host.call("test.echo", "s", [1, ["x"]]));
console.log(host.call("test.echo"));
console.log(host.wait("test.later", 0, "w", [2]));
// 真偽とオブジェクト（段階 P7）：オブジェクトは自分のプロパティだけを渡し、ホストの Dict はオブジェクトになる
class Point { constructor(x) { this.x = x; } norm() { return this.x; } }
console.log(host.call("test.echo", true, { a: 1, b: [false, "s"], c: {} }, new Point(3)));
console.log(host.wait("test.later", 0, { k: true }));
for (const f of [() => host.call("test.fail"), () => host.call("test.none"), () => host.wait("test.none"),
                 () => host.wait("test.later", -1), () => host.wait("test.fail")]) {
  try { f(); } catch (e) { console.log(String(e)); }
}
// 128 KB の結果も受け取れる（段階 P7 から。前は、パイプに入らないのでエラーだった。17 回倍にして作る。repeat は
// 1 文字ずつつなぐので、Python 版の VM では遅い）
let big = "x";
for (let i = 0; i < 17; i++) big = `${big}${big}`;
console.log(host.wait("test.later", 0, big)[1].length, big.length);

// ホストのオブジェクトを、host.wait と host.call と async 関数に、参照のまま渡す（段階 S0b。M1 のレビュー、shima3/conma#95）
const b = host.wait("test.box", 7);
console.log(host.wait("test.use", b), b.n);
console.log(host.call("test.use", b), b.n);
const c = host.wait("test.box", 1);
console.log(host.wait("test.useAll", { list: [b, c, b] }));
// 返ってきたのも、同じオブジェクト
async function echo(x) {
  x.inc();
  return x;
}
const back = echo(c);
console.log(back === c, c.n);
console.log(host.wait("test.use", 1));

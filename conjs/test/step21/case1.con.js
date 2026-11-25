// ホストの async 関数を host.wait で呼ぶ（段階 P10。case1.host.js）。待つのは呼んだ処理だけ
console.log(host.available("files"));
const slow = spawn(() => host.wait("files.later", 300, "slow"));
console.log(host.wait("files.later", 10, "fast"));
console.log(host.wait("files.countLines", "case1.host.js"));
console.log(join(slow));
try {
  host.wait("files.countLines", "none.txt");
} catch (e) {
  console.log(e instanceof Error);
}

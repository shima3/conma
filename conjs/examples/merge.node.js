// merge.con.js と同じことを Node.js で書いたもの（docs/dev/PLATFORM-PLAN-ja.md の 4.8 節、段階 P8）。ファイルを 1 行ずつ
// 読むには、readline の Interface が要り、行は非同期のイテレータから await で受け取る。2 つのファイルを比べた結果で
// 交互に進めるので、for await では書けず、2 つのイテレータの next() を、それぞれ await で呼ぶ。関数は async になる。
//   node conjs/examples/merge.node.js a.txt b.txt
// conjs/test/compare.sh が、merge.con.js と同じ出力になることを確かめる（conjs/test/node-pairs.txt）
const fs = require("fs");
const readline = require("readline");

async function main() {
  const args = process.argv.slice(2);
  if (args.length !== 2) throw new Error("usage: merge.node.js a.txt b.txt (- for standard input)");
  const open = (path) =>
    readline.createInterface({ input: path === "-" ? process.stdin : fs.createReadStream(path), crlfDelay: Infinity });
  const a = open(args[0]);
  const b = open(args[1]);
  const ia = a[Symbol.asyncIterator]();
  const ib = b[Symbol.asyncIterator]();
  const next = async (it) => {
    const r = await it.next();
    return r.done ? undefined : r.value;
  };
  try {
    let x = await next(ia);
    let y = await next(ib);
    while (x !== undefined || y !== undefined) {
      if (y === undefined || (x !== undefined && x < y)) {
        console.log(`<\t${x}`);
        x = await next(ia);
      } else if (x === undefined || y < x) {
        console.log(`>\t${y}`);
        y = await next(ib);
      } else {
        console.log(`=\t${x}`);
        x = await next(ia);
        y = await next(ib);
      }
    }
  } finally {
    a.close();
    b.close();
  }
}

main();

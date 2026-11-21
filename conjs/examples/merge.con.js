// 並べた（ソートした）2 つのファイルを 1 行ずつ読み、片方だけにある行と、両方にある行を出す（Unix の comm と同じ。
// docs/dev/PLATFORM-PLAN-ja.md の 4.8 節、段階 P8）。どちらのファイルも全部は読まず、2 つの readLine を、比べた結果で
// 交互に進める。- は標準入力。
//   conjs/conjs conjs/examples/merge.con.js a.txt b.txt
//   sort a.txt | conjs/conjs conjs/examples/merge.con.js - b.txt
// 出力の行は「<」（a だけ）、「>」（b だけ）、「=」（両方）とタブで始まる。同じことを Node.js で書いたものは merge.node.js
if (argv.length !== 2) throw new Error("usage: merge.con.js a.txt b.txt (- for standard input)");
const open = (path) => (path === "-" ? fs.stdin : fs.open(path));
const a = open(argv[0]);
const b = open(argv[1]);
let x = a.readLine();
let y = b.readLine();
while (x !== undefined || y !== undefined) {
  if (y === undefined || (x !== undefined && x < y)) {
    console.log(`<\t${x}`);
    x = a.readLine();
  } else if (x === undefined || y < x) {
    console.log(`>\t${y}`);
    y = b.readLine();
  } else {
    console.log(`=\t${x}`);
    x = a.readLine();
    y = b.readLine();
  }
}
a.close();
b.close();

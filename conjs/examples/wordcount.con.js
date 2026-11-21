// ファイルを 1 行ずつ読み、語の数の多い順に 10 個を出す（docs/dev/PLATFORM-PLAN-ja.md の 4.8 節、段階 P8）。
// fs.readLines のいちばん短い使い方の例。ファイル全体を 1 つの文字列にしないので、大きなファイルも読める。
//   conjs/conjs conjs/examples/wordcount.con.js file.txt
//   cat file.txt | conjs/conjs conjs/examples/wordcount.con.js
const lines = argv.length > 0 ? fs.readLines(argv[0]) : fs.stdin.readLines();
// 語をキーにすると、constructor などの名前を読めない（ConJS のオブジェクトにはプロトタイプのキーを置かない）ので、
// 前に「w:」を付ける
const counts = {};
let total = 0;
// 英字、数字、' の並びを 1 語とする（ConJS には正規表現がないので、1 文字ずつ見る）
function words(line) {
  const out = [];
  let word = "";
  for (const ch of `${line.toLowerCase()} `) {
    if ((ch >= "a" && ch <= "z") || (ch >= "0" && ch <= "9") || ch === "'") {
      word = `${word}${ch}`;
    } else if (word !== "") {
      out.push(word);
      word = "";
    }
  }
  return out;
}
for (const line of lines) {
  for (const word of words(line)) {
    const key = `w:${word}`;
    counts[key] = (counts[key] || 0) + 1;
    total++;
  }
}
const top = Object.entries(counts)
  .sort((p, q) => q[1] - p[1] || (p[0] < q[0] ? -1 : 1))
  .slice(0, 10);
console.log(`${total} words, ${Object.keys(counts).length} different`);
for (const [key, n] of top) console.log(`${n}\t${key.slice(2)}`);

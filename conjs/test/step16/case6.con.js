// 3 つの処理が、少しずつ届く標準入力（パイプ。case6.feed.py が、行の途中で分けて、間を空けて送る）を readLine で取り合う
// （4.8 節、段階 P8。M2 の PR shima3/conma#21 のレビュー）。読めないときに待つ間、ほかの処理が同じファイルを読んでも、
// 行は壊れない（どの行も 1 回ずつ、どこかの処理に届く）
const f = fs.stdin;
const out = [];
function reader() {
  for (;;) {
    const line = f.readLine();
    if (line === undefined) return;
    out.push(line);
  }
}
const tasks = [spawn(reader), spawn(reader), spawn(reader)];
for (const t of tasks) join(t);
const bad = out.filter((l) => l !== "x".repeat(30));
console.log(out.length, bad.length);

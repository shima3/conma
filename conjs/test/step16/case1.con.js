// ファイルを 1 行ずつ読む（docs/dev/PLATFORM-PLAN-ja.md の 4.8 節、段階 P8）：readLine は行の終わり（\n か \r\n）を除いて
// 返し、終わりなら undefined。最後の行に \n がなくても、その行を返す。read() は残りを全部。readLine の後の readLines()
const f = fs.open("case1_a.txt");
console.log(JSON.stringify([f.readLine(), f.readLine(), f.readLine()]));
console.log(JSON.stringify(f.read()), JSON.stringify(f.read()), f.readLine());
f.close();
const g = fs.open("case1_a.txt");
g.readLine();
console.log(JSON.stringify([...g.readLines()]), g.closed);
const e = fs.open("case1_empty.txt");
console.log(e.readLine(), JSON.stringify(e.read()), JSON.stringify([...fs.readLines("case1_empty.txt")]));
e.close();
// __OS_read__ は 4,096 バイトずつ読むので、読み足す所を通るファイル：区切りをまたぐ行、区切りの両側に分かれる \r\n、
// 区切りをまたぐ ASCII でない文字、4,096 バイトより長い 1 行
const pad = (n) => "x".repeat(n);
fs.writeFile("b1.txt", `${pad(4090)}\nabcdefghij\nend\n`);
fs.writeFile("b2.txt", `${pad(4095)}\r\nnext\n`);
fs.writeFile("b3.txt", `${pad(4095)}あいう\nend\n`);
fs.writeFile("b4.txt", `${pad(10000)}\nshort\n`);
for (const name of ["b1.txt", "b2.txt", "b3.txt", "b4.txt"]) {
  const lines = [...fs.readLines(name)];
  console.log(name, lines.map((l) => l.length).join(","), lines.map((l) => l.slice(-3)).join(","));
}

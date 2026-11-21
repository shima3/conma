// 標準入力を 1 行ずつ読む（4.8 節、段階 P8）：fs.stdin.readLines() で case4.in を読む。標準入力を行で読んだ後に、
// キー（nextEvent。段階 P6a）で読もうとすると Error。fs.stdout.write は改行を足さない。readLines() が終われば、fs.stdin も
// 閉じたことになる
const lines = [];
for (const line of fs.stdin.readLines()) lines.push(line);
console.log(lines);
console.log(fs.stdin.closed);           // readLines() は、終わったときに閉じる（本当の標準入力は閉じない）
try { nextEvent(); } catch (e) { console.log(`${e}`); }
fs.stdout.write("no newline,");
fs.stdout.write(" then one\n");
console.log(fs.stdin, fs.stdout);

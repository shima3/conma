// fs.writeFile と fs.readFile：ファイル全体を文字列で書き、読む（相対パスは、プログラムを動かしたディレクトリから）
fs.writeFile("out.txt", "first line\nsecond line\n");
console.log(JSON.stringify(fs.readFile("out.txt")));
fs.writeFile("out.txt", "日本語のテキスト");
const text = fs.readFile("out.txt");
console.log(text, text.length);
fs.writeFile("empty.txt", "");
console.log(JSON.stringify(fs.readFile("empty.txt")));
console.log("writeFile returns", fs.writeFile("out.txt", "x"));

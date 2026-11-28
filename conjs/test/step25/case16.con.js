// <$ [a, b, ...r]：配列 [a, b, ...r] の要素を、入力の行に（名前と ...名前 だけ。... はどこにでも）
const header = "name";
const rows = ["b", "a"];
const one = 1;
$ cat <$ [header, ...rows]
$ cat <$ [ one , header ]
let s;
$ sort <$ [...rows, header] >$ [...s]
console.log(s);
// コマンド型関数とパイプ
function f(args, input, output) {
  for (const x of input) output.push(`<${x}>`);
}
$ $f <$ [header, one] | cat
// 同じ名前に戻す
let r = ["z", "y"];
$ sort <$ [...r] >$ [...r]
console.log(r);

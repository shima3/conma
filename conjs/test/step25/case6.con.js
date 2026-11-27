// 文字列、テンプレートリテラル、コメントの中の行は、コマンド行でない
const t = `
$ echo not a command in a template
${"x"}
$ echo nor here`;
/*
$ echo not in a comment
*/
// $ echo not in a line comment
console.log(t.split("\n").length);
const u = `a ${ { k: `b
$ c` }.k } d`;
console.log(u.length);
$ echo but this is one
let n = 0;
if (n === 0)
  $ echo the body of if
for (const w of ["x", "y"]) {
  $ echo ${w} # a comment after a space
}
// 行の終わりの \ で次の行に続く文字列の中の行も、コマンド行でない（M1 のレビュー、shima3/conma#96）
const s = "x\
$ y";
console.log(s);
// \\ で終わる行は、次の行に続かない（\ が 1 つの語）。\ が奇数なら続く
$ echo a\\
$ echo b \
  c

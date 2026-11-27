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

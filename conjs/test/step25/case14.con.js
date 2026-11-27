// コマンドの出力を 1 つの値に：>$ の Queue の text()（Bash の $(...) と同じく、終わりの改行を除く）
const lines = ["b", "a", "c"];
let n;
$ wc -l <$ lines >$ n
console.log(Number(n.text().trim()));
let s;
$ printf "%s\n\n" ${lines} >$ s
console.log(JSON.stringify(s.text()));
let t;
$ sort <$ lines >$ t
console.log(t.text().split("\n").join(" "));

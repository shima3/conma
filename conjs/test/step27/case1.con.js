// ファイルへのリダイレクト（docs/dev/CONJS-SHELL-PLAN.md の 21 節）：< > >> 2> 2>>。外部コマンドには、ホストが開いたファイルを
// そのまま渡し、コマンド型関数は fs.open で読み書きする
const name = "out.txt";
$ printf "b\na\n" > ${name}
$ sort < out.txt > sorted.txt
$ cat sorted.txt
// >> は足し、> は書き直す
$ echo c >> sorted.txt
$ cat < sorted.txt | wc -l | tr -d " "
$ echo new > out.txt
$ cat out.txt
// 標準エラー：2> と 2>>
$ sh -c "echo e1 1>&2; echo o1" 2> err.txt > o.txt
$ sh -c "echo e2 1>&2" 2>> err.txt
$ cat err.txt o.txt
// パイプの初めの < と最後の >、途中の 2>
$ cat < sorted.txt | sh -c "cat; echo e3 1>&2" 2> err2.txt | sort -r > rev.txt
$ cat rev.txt err2.txt
// コマンド型関数：ファイルの行が入力に、出力の要素がファイルの行に
function up(args, input, output) {
  for (const x of input) output.push(x.toUpperCase());
}
$ $up < sorted.txt > up.txt
$ cat up.txt
$ printf "d\n" | $up >> up.txt
let u;
$ cat < up.txt | $up >$ [...u]
console.log(u);
// 要素はそのまま String(x) にして 1 行ずつ
function nums(args, input, output) {
  output.push(1);
  output.push([2, 3]);
}
$ $nums > n.txt
console.log(JSON.stringify(fs.readFile("n.txt")));
// <$ や >$ と組み合わせる
const lines = ["y", "x"];
$ sort <$ lines > l.txt
let back;
$ cat < l.txt >$ [...back]
console.log(back);

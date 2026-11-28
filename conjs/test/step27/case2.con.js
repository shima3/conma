// 開けないファイル：その行の失敗（|| が受け、標準エラーに Bash と同じ形で出す）。コマンドは動かない
$ cat < nofile.txt || echo recovered
$ echo x > nodir/x.txt || echo recovered
function up(args, input, output) {
  for (const x of input) output.push(x.toUpperCase());
}
$ $up < nofile.txt || echo recovered
// >$ のキューは閉じる（読み手は終わる）
let q;
$ $up < nofile.txt >$ q || echo recovered
console.log([...q]);
// となりあう外部コマンドは一緒に始めるので、どれかのファイルを開けなければ、どれも動かない
$ cat < nofile.txt | wc -l || echo recovered
// パイプのコマンド型関数は、Bash と同じく動く（M1 のレビュー、shima3/conma#102）。結果は最後のコマンドのもの
function note(args, input, output) {
  console.log("note runs");
  for (const x of input) output.push(x);
}
$ printf "a\n" > in.txt
$ $note < in.txt | $note > nodir/x.txt || echo recovered
$ cat < nofile.txt | $note
// 出力を開けないとき、開いた入力のファイルは閉じる
$ $note < in.txt > nodir/x.txt || echo recovered
try {
  $ cat < nofile.txt
} catch (e) {
  console.log(e.message);
}

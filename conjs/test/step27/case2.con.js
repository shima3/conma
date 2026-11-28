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
// パイプでは、開けなければ、どのコマンドも動かない
$ cat < nofile.txt | wc -l || echo recovered
try {
  $ cat < nofile.txt
} catch (e) {
  console.log(e.message);
}

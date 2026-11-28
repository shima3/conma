// Bash に合わせる（M1 のレビューと、ユーザーの答え。shima3/conma#97）：関数の例外は、コマンドの失敗として || が受ける。
// パイプの結果は最後のコマンドのもの（pipefail なし）。投げずに済ませた誤り（CommandError のほか）は、標準エラーに出す
function boom(args, input, output) {
  throw new Error("boom");
}
console.log("1:");
$ $boom || echo fallback
console.log("2:");
$ $boom | cat || echo fallback
console.log("3:");
$ no-such-cmd || echo fallback
console.log("4:");
$ no-such-cmd | cat
console.log("5:");
$ false || echo after-false
console.log("6:");
try {
  $ $boom
} catch (e) {
  console.log("thrown:", e.message);
}
console.log("7:");
try {
  $ true | $boom
} catch (e) {
  console.log("thrown:", e.message);
}

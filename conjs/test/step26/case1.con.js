// パイプ（docs/dev/CONJS-SHELL-PLAN.md の段階 S3）：外部コマンドどうしは OS のパイプで、コマンド型関数とは Queue でつなぐ
$ printf "%s\n" b a c | sort
let out;
$ printf "%s\n" b a c | sort | head -2 >$ out
console.log([...out].join(","));
function upper(args, input, output) {
  for (const x of input) output.push(x.toUpperCase());
}
function count(args, input, output) {
  let n = 0;
  for (const x of input) n = n + 1;
  output.push(`${args[0]}${n}`);
}
$ printf "%s\n" x y | $upper | sort -r
const words = ["p", "q", "r"];
$ $upper <$ words | $count "n="
// 読むのをやめるコマンド：前のコマンドは SIGPIPE で、前の関数は書くのをやめる
$ yes | head -2
function* nat() {
  let i = 0;
  for (;;) {
    yield i;
    i = i + 1;
  }
}
const g = nat();
function double(args, input, output) {
  for (const x of input) output.push(x * 2);
}
$ $double <$ g | head -3
console.log("after head");
// 大きな出力も、上限のあるキューを通る
let lines;
$ seq 1 20000 | $upper | wc -l >$ lines
console.log([...lines].join("").trim());
// 結果は、最後のコマンドのもの
$ false | true
console.log("false | true succeeds");
try {
  $ true | false
} catch (e) {
  console.log(e.name, e.status);
}

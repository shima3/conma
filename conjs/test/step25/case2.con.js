// コマンド型関数（$ $名前。D3）と、>>$ の順
function upper(args, input, output) {
  for (const x of input) output.push(`${args[0]}${x.toUpperCase()}`);
}
const words = ["a", "b"];
let res;
$ $upper "> " <$ words >$ res
console.log([...res].join("|"));
// 出力のリダイレクトがなければ、標準出力に 1 行ずつ
$ $upper "* " <$ words
// コマンド型関数の出力を、外部コマンドの入力に
let up;
$ $upper "" <$ words >$ up
$ sort -r <$ up
// 要素は、そのまま渡る
function same(args, input, output) {
  for (const x of input) output.push(x);
}
const o = { k: 1 };
const objs = [o, 2];
let back;
$ $same <$ objs >$ back
const got = [...back];
console.log(got[0] === o, got[1]);
// 関数が投げても、出力は閉じる
function fail(args, input, output) {
  output.push("partial");
  throw new Error("failed in the function");
}
let part;
try {
  $ $fail >$ part
} catch (e) {
  console.log(e.message, part.closed, [...part].join(","));
}
// >>$：後から始めた行の出力は、先の行の出力の後に読まれる（先の行が終わる前に始めても）
let q;
$ echo one >$ q
const t = spawn(() => {
  $ sh -c "sleep 0.2; echo two" >>$ q
});
sleep(10);
$ echo three >>$ q
join(t);
console.log([...q].join(","));
// $ だけの名前は、外部コマンド（D3）：ConJS の sleep があっても、OS の sleep
$ sleep 0
console.log(typeof sleep);

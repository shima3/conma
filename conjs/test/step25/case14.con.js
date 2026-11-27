// >$ と 2>$ の後の [a, b, ...r]：コマンドが終わったら、出力の行の配列を分割代入する（docs/dev/CONJS-SHELL-PLAN.md の 20 節）
let n;
$ printf "3\n" >$ [n]
console.log(Number(n));
let first, second, rest;
$ printf "a\nb\nc\nd\n" >$ [first, second, ...rest]
console.log(first, second, rest);
// 全体を配列で：何度読んでも同じ
const lines = ["b", "a", "c"];
let s;
$ sort <$ lines >$ [...s]
console.log(s.join(" "), [...s].length, [...s].length, Array.isArray(s));
// 穴、行が足りないとき、途中の空の行
let x, y;
$ printf "1\n2\n3\n" >$ [, x, , y]
console.log(x, y);
let p;
$ printf "%s\n\n" b a c >$ [...p]
console.log(JSON.stringify(p));
// || が受けた失敗でも、出力は入る（grep -c は数を出して、ステータス 1 で終わる）
let c;
$ grep -c zz <$ lines >$ [c] || echo none
console.log(c);
// パイプの最後と、2>$ [...]（どの外部コマンドにも）
let e, o;
$ sh -c "echo out; echo err 1>&2" 2>$ [e] | cat >$ [o]
console.log(o, e);
// コマンド型関数の出力：要素はそのまま
const obj = { k: 1 };
function f(args, input, output) {
  output.push(obj);
  for (const v of input) output.push(v.toUpperCase());
}
let u0, u;
$ printf "a\nb\n" | $f >$ [u0, ...u]
console.log(u0 === obj, u);
// 動かないものの出力は入らない
let z = "before";
$ false && echo x >$ [z] || true
console.log(z);
// >$ に名前だけなら、今までどおり Queue
let q;
$ printf "a\n" >$ q
console.log(typeof q, [...q]);
// <$ と同じ名前に出力を入れる：入力は、出力の用意より先に読む（M1 のレビュー、shima3/conma#99）
let srt = ["b", "a"];
$ sort <$ srt >$ [...srt]
console.log(srt);
let sq = ["d", "c"];
$ sort <$ sq >$ sq
console.log([...sq]);
function up(args, input, output) {
  for (const v of input) output.push(v.toUpperCase());
}
let fk = ["x"];
$ $up <$ fk >$ [...fk]
console.log(fk);
let pr = ["q", "p"];
$ false || sort <$ pr | cat >$ [...pr]
console.log(pr);

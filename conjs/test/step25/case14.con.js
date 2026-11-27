// >@ と 2>@：コマンドが終わったら、出力の行の配列を変数に入れる（キューでなく配列なので、何度でも読め、n[0] で取れる）
let n;
$ printf "3\n" >@ n
console.log(Number(n[0]), n.length, Array.isArray(n));
// 何度読んでも同じ
const lines = ["b", "a", "c"];
let s;
$ sort <$ lines >@ s
console.log(s.join(" "), [...s].length, [...s].length);
// 途中の空の行も要素（終わりの改行は行の区切り）
let p;
$ printf "%s\n\n" b a c >@ p
console.log(JSON.stringify(p));
// || が受けた失敗でも、出力は入る（grep -c は数を出して、ステータス 1 で終わる）
let c;
$ grep -c zz <$ lines >@ c || echo none
console.log(c);
// パイプの最後と、2>@（どの外部コマンドにも）
let e;
$ sh -c "echo out; echo err 1>&2" 2>@ e | cat >@ s
console.log(s, e);
// コマンド型関数の出力：要素はそのまま
const o = { k: 1 };
function f(args, input, output) {
  output.push(o);
  for (const x of input) output.push(x.toUpperCase());
}
let u;
$ printf "a\nb\n" | $f >@ u
console.log(u.length, u[0] === o, u[1], u[2]);
let v;
$ $f <$ lines >@ v
console.log(v.length, v[3]);
// 動かないものの >@ は作らない
let z = "before";
$ false && echo x >@ z || true
console.log(z);

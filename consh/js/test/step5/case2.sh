# 複数の行、ブロック、await、イテレータ、出力の規則
{
  let total = 0
  for (const x of [1, 2, 3]) {
    total += x
  }
  t = total
}
echo "t=$t"
s = [1,
  2,
  3].join("")
echo "s=$s"
r = await new Promise(res => setTimeout(() => res("resolved"), 10))
echo "r=$r"
new Promise(res => res("promise value"))
`template ${1 + 1} with ${"nested ${braces}"}`
["abbc", "ac"].map(s => /ab+c/.test(s)).join(" ")
console.log("a/b")
String("x").repeat(3).length / 3
[1, 2].values()
z = 5
z
let y = 1; y + 1
echo "y=$y"
# 関数の本体の中の JS の文。変数はその関数の局所変数
k = 1
function f (a) {
  k = Number(a) * 10
  echo "in f: $k"
}
f 4
echo "k=$k"
# パイプラインの中の関数でも動く。stdin と console も使える。JS の文の中の ; は JavaScript のもの
function count () { c = 0; { for await (const l of stdin) c++ }; console.log("count", c) }
seq 1 5 | count

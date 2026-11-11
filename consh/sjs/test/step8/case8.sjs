// Ref のない const（宣言より前に読まれない const は、初期値そのもの）と、そうでない const、数のリテラルとの ===、
// 例外を投げない文（行を記録しない）の後のエラーの行
const show = (x) => console.log(JSON.stringify(x))
// 関数の宣言（ブロックの始めで作る）が読む const
function readLater() { return late }
const late = "late"
show(readLater())
// 前の文の関数が読む const
const early = () => after
const after = 3
show(early())
// 自分を読む const
const fact = (n) => n <= 1 ? 1 : n * fact(n - 1)
show(fact(5))
// ループの中の const、閉包が捕まえる const
const fs = []
for (let i = 0; i < 3; i++) {
  const sq = i * i
  const label = `i${i}`
  fs.push(() => [label, sq])
}
show(fs.map((f) => f()))
// 同じ名前の外の変数と内の const
let x = 1
{
  show(x)
  const y = x + 1
  show(y)
}
{
  const x = 10
  show(x)
}
show(x)
// switch の中の const
for (const k of [1, 2]) {
  switch (k) {
    case 1: { const m = "one"; show(m); break }
    case 2: const m2 = "two"; show(m2)
  }
}
// try と catch と finally の中の const
try {
  const t = "try"
  show(t)
  throw `${t}!`
} catch (e) {
  const c = `${e}?`
  show(c)
} finally {
  const f = "finally"
  show(f)
}
// ジェネレータの yield をはさむ const
function* gen() {
  const a = yield 1
  const b = a * 2
  yield b
}
const it = gen()
show([it.next().value, it.next(5).value])
// 数のリテラルとの === と !==（NaN、Infinity、-0、文字列、小数）
const values = [0, -0, 1, NaN, Infinity, -Infinity, "0", null, 1.5]
show(values.map((v) => [v === 0, v !== 0, v === 1, v !== 1, v === 1.5, v !== 1.5]))
let n = 3
let count = 0
while (n !== 0) { n -= 1; count += 1 }
show(count)
// 例外を投げない文の後のエラーの行
let acc = 0
acc += 1
acc.foo.bar = 1

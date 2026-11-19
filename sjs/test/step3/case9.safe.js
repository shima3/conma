// 段階 B9：ジェネレータの中と、await の後の受け手のない例外
function* g() {
  yield 1
  const x = null
  yield x.y
}
const it = g()
console.log(it.next().value)
async function a() {
  await 1
  return [].nothing.at
}
a()
console.log("after a")
it.next()

// async ジェネレータの順序：Node.js と同じマイクロタスクの順に動く
const log = []
const p = (tag) => log.push(tag)
async function* a() { p("a0"); await null; p("a1"); yield "A1"; p("a2"); await null; yield "A2"; p("a3") }
async function* b() { p("b0"); yield "B1"; await null; p("b1"); yield "B2" }
async function run(g, name) { for await (const x of g) { p(`${name} got ${x}`); await null } p(`${name} end`) }
run(a(), "ra"); run(b(), "rb")
Promise.resolve().then(() => p("t1")).then(() => p("t2")).then(() => p("t3")).then(() => p("t4")).then(() => p("t5"))
async function* c() {
  try { yield 1; yield 2 } catch (e) { p(`c caught ${e}`); yield 99 } finally { await null; p("c finally") }
}
async function main() {
  const g = c()
  p(JSON.stringify(await g.next()))
  p(JSON.stringify(await g.throw("x")))
  p(JSON.stringify(await g.next()))
  const h = c(); await h.next(); p(JSON.stringify(await h.return(7)))
  function* sync() { try { yield 1; yield Promise.resolve(2); yield 3 } finally { p("sync finally") } }
  for await (const v of sync()) { p(`v ${v}`); if (v === 2) break }
  async function* rej() { try { yield Promise.reject("bad") } catch (e) { p(`rej caught ${e}`); yield "after" } }
  for await (const r of rej()) p(`r ${r}`)
  const d = c(); const all = await Promise.all([d.next(), d.return(5), d.next()]); p(JSON.stringify(all))
}
main().then(() => console.log(log.join("\n")), (e) => console.log("failed", e, log.join("\n")))

// async ジェネレータ（async function*、yield、yield*、return、throw）と for await
const log = []
async function* count(n) {
  try {
    for (let i = 0; i < n; i++) {
      log.push(`yield ${i}`)
      const got = yield i
      if (got !== undefined) log.push(`got ${got}`)
    }
    return "end"
  } finally {
    log.push("finally")
  }
}
async function main() {
  for await (const x of count(3)) log.push(`x ${x}`)
  const g = count(5)
  const r = await Promise.all([g.next(), g.next("a"), g.next("b")])
  log.push(JSON.stringify(r))
  log.push(JSON.stringify(await g.return(42)), JSON.stringify(await g.next()))
  for await (const v of [1, Promise.resolve(2), 3]) log.push(`v ${v}`)
  for await (const y of count(10)) { if (y === 1) break; log.push(`y ${y}`) }
  const h = async function* () { const a = await Promise.resolve(10); yield a; yield* [a + 1, Promise.resolve(a + 2)]; const b = yield* count(1); log.push(`b ${b}`); return Promise.resolve("done") }
  const out = []; for await (const z of h()) out.push(z)
  log.push(out.join(","))
  const it = h(); log.push(JSON.stringify(await it.next()), JSON.stringify(await it.throw("boom").catch((e) => e)))
  const e = async function* () { throw "early" }
  try { for await (const q of e()) log.push(q) } catch (err) { log.push(`caught ${err}`) }
  const s = count(2); log.push(JSON.stringify(await s.return("r0")), JSON.stringify(await s.next()))
  log.push(typeof s, String(s))
  const o = { async *m() { yield 1 } }
  for await (const w of o.m()) log.push(`w ${w}`)
}
main().then(() => console.log(log.join("\n")))
const order = []
async function* tick() { order.push("a"); yield 1; order.push("b") }
const t = tick(); t.next().then((r) => order.push(`r ${r.value}`)); order.push("sync")
Promise.resolve().then(() => order.push("p1")).then(() => order.push("p2")).then(() => order.push("p3")).then(() => console.log(order.join(" ")))

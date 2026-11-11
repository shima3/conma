// the order of microtasks
const log = (s) => console.log(s)
async function a1() { log("a1 start"); await a2(); log("a1 end") }
async function a2() { log("a2") }
log("script start")
a1()
Promise.resolve().then(() => log("p1")).then(() => log("p2")).then(() => log("p3"))
log("script end")
// an async function that returns a promise takes two more steps
async function r1() { return Promise.resolve("r1") }
async function r2() { return "r2" }
r1().then(log)
r2().then(log)
Promise.resolve().then(() => log("t1")).then(() => log("t2")).then(() => log("t3")).then(() => log("t4"))
// await in loops and recursion
async function sum(xs) { let s = 0; for (const x of xs) s += await x; return s }
sum([1, Promise.resolve(2), 3]).then((v) => log(`sum ${v}`))
async function fact(n) { return n <= 1 ? 1 : n * await fact(n - 1) }
fact(10).then((v) => log(`fact ${v}`))
// Promise.all rejects with the first rejection
Promise.all([Promise.resolve(1), Promise.reject("bad"), Promise.reject("worse")]).then(() => log("no"), (e) => log(`all rejected: ${e}`))
Promise.all([]).then((v) => log(`empty all ${JSON.stringify(v)}`))
// a generator inside an async function
async function gens() { const out = []; for (const x of (function* () { yield 1; yield 2 })()) out.push(await x * 10); return out }
gens().then((v) => log(JSON.stringify(v)))
// then with non-functions passes values through
Promise.resolve(7).then(null).then(undefined, null).then((v) => log(`passed ${v}`))
// a cycle
const cyc = Promise.resolve().then(() => cyc)
cyc.catch((e) => log(`${e}`))

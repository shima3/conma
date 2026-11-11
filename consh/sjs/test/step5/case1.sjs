// async and await, then, catch, finally, Promise.resolve, reject, all
async function f(x) { console.log("f start", x); const y = await x; console.log("f after await", y); return y * 2 }
const p = f(1)
console.log("sync after f", typeof p, String(p))
p.then((v) => console.log("then", v))
Promise.resolve(5).then((v) => { console.log("resolved", v); return Promise.resolve(v + 1) }).then((v) => console.log("chained", v))
const g = async () => { try { await Promise.reject("no") } catch (e) { console.log("caught", e) } finally { console.log("finally") } return "g done" }
g().then(console.log)
Promise.all([f(10), 3, g()]).then((vs) => console.log("all", JSON.stringify(vs)))
const o = { async m(a) { return (await a) + 1 } }
o.m(Promise.resolve(41)).then((v) => console.log("method", v))
async function thrower() { await null; throw new_err }
thrower().catch((e) => console.log(`catch: ${e}`))
Promise.reject("x").finally(() => console.log("fin")).catch((e) => console.log("after finally", e))
console.log("end of the program")

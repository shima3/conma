// for await：投げて終わった async ジェネレータの return() は呼ばない（Node.js と同じ順）
const log = []
async function* e() { log.push("e start"); throw "boom" }
async function m() { try { for await (const x of e()) log.push(x) } catch (err) { log.push(`caught ${err}`) } }
m(); Promise.resolve().then(() => log.push("t1")).then(() => log.push("t2")).then(() => log.push("t3")).then(() => log.push("t4")).then(() => console.log(log.join(" ")))

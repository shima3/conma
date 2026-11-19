// 段階 B9：投げずに拒否した Promise は位置を出さない（前に受けた async の例外の位置を出さない）
const q = async () => { await 1; throw "late" }
q().catch((e) => console.log(`handled ${e}`))
const r = async () => { await 2 }
r().then(() => Promise.reject("plain"))

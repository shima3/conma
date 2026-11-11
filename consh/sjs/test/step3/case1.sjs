// throw と catch：文字列、オブジェクト、実行時のエラー（name、message、String、JSON）
try { throw "boom" } catch (e) { console.log("caught", e) }
try { throw { code: 7 } } catch (e) { console.log(e.code, JSON.stringify(e)) }
try { const x = 1 + "a" } catch (e) { console.log(e.name, e.message, `${e}`, JSON.stringify(e), Object.keys(e)) }
try { null.x } catch (e) { console.log(e.message) }
try { nosuch } catch (e) { console.log(`${e}`) }
try { } catch { }
try { throw 1 } catch { console.log("no param") }
const deep = (n) => { if (n === 0) throw "bottom"; return deep(n - 1) }
try { deep(50) } catch (e) { console.log("deep", e) }
try { [1, 2].map((x) => { if (x === 2) throw `bad ${x}`; return x }) } catch (e) { console.log(e) }
let r = []
for (const v of [1, 2, 3]) { try { if (v === 2) throw "two"; r.push(v) } catch (e) { r.push(e) } }
console.log(r)
try { try { throw "inner" } catch (e) { throw `re-${e}` } } catch (e) { console.log(e) }

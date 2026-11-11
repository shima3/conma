// finally：ふつうに終わったとき、例外、return、break、continue
try { console.log("body") } finally { console.log("finally 1") }
const f = () => { try { return "from try" } finally { console.log("finally 2") } }
console.log(f())
const over = () => { try { return 1 } finally { return 2 } }
console.log(over())
const g = () => { for (let i = 0; i < 5; i++) { try { if (i === 1) continue; if (i === 3) break; console.log("i", i) } finally { console.log("fin", i) } } return "done" }
console.log(g())
try { try { throw "inner" } finally { console.log("inner finally") } } catch (e) { console.log("outer caught", e) }
const h = () => { try { try { return "x" } finally { console.log("a") } } finally { console.log("b") } }
console.log(h())
const k = () => { try { throw "t" } catch (e) { return `c ${e}` } finally { console.log("after catch") } }
console.log(k())

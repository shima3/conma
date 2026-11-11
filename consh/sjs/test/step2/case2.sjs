// オブジェクト：リテラル、メンバー、代入、メソッド、短縮形、計算したキー、Object と JSON
const k = "dyn"
const n = 5
const o = { n, s: "x", nested: { list: [1, "two"] }, [k]: true, "a-b": 1, 2: "two"}
console.log(o.n, o.s, o.nested.list[1], o[k], o["a-b"], o[2])
o.n += 5
o.n++
o["s"] = "y"
o.added = null
console.log(o.n, o.s, o.added, o.missing)
console.log(Object.keys(o), Object.values({ p: 1, q: [2] }), Object.entries({ p: 1 }))
console.log(JSON.stringify({ a: [1, "s", null, true], b: { c: undefined }, d: 1.5 }), JSON.stringify("q\"uote"), JSON.stringify([undefined, () => 1]))
console.log(o?.nested?.list, o?.nope?.x, null?.y, undefined?.[0])
const counter = { count: 0, inc() { counter.count += 1; return counter.count } }
console.log(counter.inc(), counter.inc(), counter.count)

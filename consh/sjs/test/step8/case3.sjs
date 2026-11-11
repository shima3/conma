// 凍らせたタグ付きテンプレートの配列、組み込みと関数のメンバー（Object.keys）、穴の多い配列（長さだけを大きくしたもの）、for (;;) の初期化の中の in
const f = (s) => s
const s = f`a${1}b`
s[0] = "x"; s.length = 0; s.y = 1; s[5] = 2
console.log(JSON.stringify(s), s.length, s.y, JSON.stringify(Object.keys(s)), JSON.stringify(Object.keys(s.raw)), "raw" in s)
for (const op of [() => s.push(1), () => s.pop(), () => s.reverse(), () => s.raw.push(1)]) { try { op() } catch (e) { console.log(e.name, e.message) } }
console.log(JSON.stringify(s.map((x) => `${x}!`)), JSON.stringify(s.slice(0, 1).concat([1])))
const t = (q) => q; const sites = []; for (let i = 0; i < 2; i++) sites.push(t`same`); console.log(sites[0] === sites[1], sites[0].length)
console.log(Object.keys(Math).length, JSON.stringify(Object.keys(Number)), JSON.stringify(Object.keys(JSON)), "isInteger" in Number, "floor" in Math)
const g = (x) => x; g.y = 1; g.z = undefined
console.log(JSON.stringify(Object.keys(g)), JSON.stringify(Object.values(g)), "y" in g, "z" in g, "length" in g, g.y)
const big = []; big.length = 2000000; big[5] = "a"; big[100000] = "b"; big.x = 1
const seen = []; big.forEach((v, i) => seen.push(`${i}:${v}`))
console.log(seen.join(), JSON.stringify(Object.keys(big)), big.indexOf("b"), big.reduce((a, b) => `${a}${b}`), big.map((v) => v).length, big.filter(() => true).length)
const k = []; for (const i in big) k.push(i); console.log(k.join())
big.length = 6; console.log(big.length, big[100000], JSON.stringify(Object.keys(big)))
big.length = 0; console.log(big.length, big.x)
const r = []; r[3] = 1; r[70] = 2; r.length = 200; r.reverse(); console.log(JSON.stringify(Object.keys(r)))
const o = { a: 1 }
for (let i = 0, ok = ("a" in o); i < 1; i++) console.log(ok, JSON.stringify([("a" in o)]), `${"a" in o}`, ((k) => k in o)("a"))
for (const x of ["a" in o]) console.log(x)

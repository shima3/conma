// 配列の添字でないメンバー（a.x、a[-1]、a[1.5]）と、Object.keys、values、entries（配列、文字列、数など）
const a = [1, , 3]
a.x = "ex"; a[-1] = "neg"; a[1.5] = "frac"; a["y"] = 2
console.log(a.x, a[-1], a[1.5], a.y, a.length, a[1], JSON.stringify(a), "x" in a, "-1" in a, "z" in a)
console.log(JSON.stringify(Object.keys(a)), JSON.stringify(Object.values(a)), JSON.stringify(Object.entries(a)))
const k = []; for (const i in a) k.push(i); console.log(k.join())
let i = -1; console.log(a[i], a[i + 0.5])
a.push = 7; console.log(a.push, a.length)
console.log(JSON.stringify(Object.keys("abc")), JSON.stringify(Object.values("ab")), JSON.stringify(Object.keys(5)), JSON.stringify(Object.keys(true)), JSON.stringify(Object.keys((x) => x)))
console.log(JSON.stringify(Object.entries({ p: 1, q: [2] })))
for (const bad of [null, undefined]) { try { Object.keys(bad) } catch (e) { console.log(e.name, e.message) } }
const t = ((s) => s)`q${1}r`; console.log(t.raw.length, "raw" in t, JSON.stringify(Object.keys(t)))

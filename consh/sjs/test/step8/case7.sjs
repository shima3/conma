// 数の添字への代入（あれば入れ替え、なければ延ばす。負、整数でない数、-0、凍った配列）、オブジェクトの名前、for...of と for...in の条件
const show = (x) => console.log(JSON.stringify(x))
const a = [1, 2, 3]
for (let i = 0; i < 3; i++) a[i] = a[i] * 10
show(a)
let k = 5
a[k] = 60
show([a, a.length, 4 in a, 5 in a])
a[3] = 40
show([a, 3 in a, 4 in a])
k = -1
a[k] = "neg"
k = 1.5
a[k] = "half"
show([a.length, a[-1], a[1.5], Object.keys(a)])
k = -0
a[k] = "zero"
show([a[0], a.length])
const t = `${1}`
const frozen = ((s) => s)`x${t}y`
let z = 0
frozen[z] = "changed"
z = 9
frozen[z] = "new"
show([frozen, frozen.length])
const o = { b: 2, a: 1 }
k = 1
o[k] = "one"
show([o.b, o.a, o.c, o[1], o["1"], Object.keys(o)])
show([Math.sqrt(16), Math.PI > 3, Math.nope])
const s = []
for (const x of [3, 0, "", null, false]) s.push(x)
for (const name in { p: 0, q: false }) s.push(name)
for (const c of "ab") s.push(c)
show(s)

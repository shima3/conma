// 配列の穴とメソッド（forEach などは飛ばし、map は残し、find は undefined とみる）、length への代入
const a = [1, , 3, , 5]
const seen = []
a.forEach((v, i) => seen.push(i))
console.log(seen.join(), JSON.stringify(a.map((v) => v * 2)), a.map((v) => v * 2).length)
console.log(a.filter(() => true).length, a.reduce((x, y) => x + y), a.some((v) => v === undefined), a.every((v) => v !== undefined))
console.log(a.indexOf(undefined), a.includes(undefined), a.find((v) => v === undefined), a.findIndex((v) => v === undefined))
const k = []; for (const i in a) k.push(i); console.log(k.join())
console.log(JSON.stringify(a.slice(1, 4)), a.slice(1, 4).length, JSON.stringify([0].concat(a, [, 7])), [0].concat(a, [, 7]).length)
const r = [1, , 3, 4]; r.reverse(); console.log(JSON.stringify(r))
const b = [1, 2, 3, 4, 5]
b.length = 2; console.log(JSON.stringify(b), b.length)
b.length = 4; console.log(JSON.stringify(b), b.length, b.indexOf(undefined))
b.length = 0; console.log(JSON.stringify(b))
for (const bad of [-1, 1.5, NaN, Infinity, 2 ** 32]) { try { b.length = bad } catch (e) { console.log(e.name, e.message) } }

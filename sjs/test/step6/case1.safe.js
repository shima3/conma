// コンマ演算子と配列の穴
const a = [1, , 3], b = [, ], c = [1, 2, ,]
console.log(a.length, b.length, c.length, a[1], JSON.stringify(a), JSON.stringify([...a]))
let x, y
;[x, , y] = [10, 20, 30]
const [p, , q] = [1, 2, 3]
console.log(x, y, p, q)
let i = 0, j = 10, s = 0
for (i = 0, j = 10; i < j; i++, j--) s += i * j
console.log(s, (1, 2, 3), i, j)
const f = () => (s++, s * 2)
console.log(f(), s)

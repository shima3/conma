// 配列：リテラル、添字、長さ、メソッド
const a = [1, 2, 3]
console.log(a, a.length, a[0], a[5], a[a.length - 1])
a.push(4, 5)
console.log(a, a.pop(), a)
console.log(a.map((x) => x * 2), a.filter((x) => x % 2 === 0), a.reduce((s, x) => s + x, 0), a.reduce((s, x) => s * x))
console.log(a.slice(1, 3), a.slice(-2), a.indexOf(3), a.indexOf(9), a.includes(2), a.join("-"))
console.log(a.some((x) => x > 3), a.every((x) => x > 0), a.find((x) => x > 1), a.findIndex((x) => x > 1))
console.log([3, 1, 2].reverse(), [1].concat([2, 3], 4), [].length, [[1, 2], [3]][1][0])
const b = []
b[2] = "x"
console.log(b.length, b[0], b[2])
a.forEach((x, i) => { if (i < 2) console.log(i, x) })

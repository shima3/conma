// 1 つの宣言の中の重なり
console.log("not reached")
const [p, { q: p }] = [1, { q: 2 }]

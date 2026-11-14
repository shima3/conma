// console.log は Node.js の util.inspect と同じ形（consh/include/sjs_inspect.se。2026-11-14 から）：引用符、-0、名前、深さ、関数、配列の穴、
// 100 個を超える要素、80 文字での折り返し、7 個以上の配列の列、Promise、長い文字列の分け方、循環
const f = function named() {}
const g = () => 1
console.log(-0, [ -0 ], 1.5, "s", ["s", 'it\'s', 'a"b', `a'b"c`], true, null, undefined, [undefined, null])
console.log({ a: 1, "b-c": 2, x: { y: { z: { w: 1 } } } }, [[1, [2, [3, [4]]]]])
console.log(f, g, [f, g], { f }, (x) => x)
console.log([1, , 3], [ , , ], [1, 2, 3, , , ,])
console.log([], {}, [[]], [{}], { a: [] })
const range = (n, k) => { const a = []; for (let i = 0; i < n; i++) a.push(i * k); return a }
console.log(range(7, 1))
console.log(range(30, 37))
console.log(range(120, 1))
console.log({ aaaaaaaaaaaa: 1, bbbbbbbbbbbbbb: 2, cccccccccccccc: 3, dddddddddddddd: 4, eeeeeeeeee: 5 })
console.log(["aaaaaaaaaaaaaaaaaaaa", "bbbbbbbbbbbbbbbbbbbb", "cccccccccccccccccccc", "dddddddddddd"])
console.log({ a: { b: { c: {} } }, arr: [1, 2, { x: [1, 2] }] })
console.log("x", 1, { a: "q" }, ["a\nb"], "a\nb")
console.log(Promise.resolve(3), [Promise.resolve({ a: 1 })])
console.log(["a", "bb", "ccc", "dddd", "eeeee", "ffffff", "ggggggg", "hhhhhhhh"])
console.log([1.5, -2, 300, 4e21, 0.001, 7, 8, 9, 10])
let xs = ""
for (let i = 0; i < 100; i++) xs = `${xs}x`
console.log({ k: xs })
console.log(["line one\nline two that is long enough to be split over more than one line of the output"])
const o = { name: "o" }
o.self = o
console.log(o, [o])
console.log({ "": 1, "a b": 2, _x: 3, $y: 4, "9z": 5 })
console.log([[1, 2, [3, 4, [5, 6, [7]]]], { a: { b: [1, { c: 2 }] } }])

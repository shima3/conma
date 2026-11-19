// タグ付きテンプレート、String.raw、文字列の逃がし（\x、\u{}、\b、\f、\v）と JSON の制御文字
const tag = (s, ...v) => JSON.stringify([s, s.raw, v])
console.log(tag`a${1}b\n${2}c\x41`)
console.log(tag`plain`, tag``)
console.log(String.raw`x\ny${1 + 1}zA`)
const sites = []
const f = (s) => s
for (let i = 0; i < 2; i++) sites.push(f`same`)
console.log(sites[0] === sites[1], f`same` === sites[0])
const o = { t: (s, x) => `${s.length}:${x}` }
console.log(o.t`a${5}b`, (tag)`q`)
console.log(JSON.stringify(["\x41\u{1F600}\u{41}\b\f\v", `\x42`]))
console.log("\u0041")
const o2 = {"\0k": 1}; console.log(JSON.stringify(o2), `a\b${1}\v`.length, String.raw`\b`.length, JSON.stringify(["\x01x\x02", "\b"]))

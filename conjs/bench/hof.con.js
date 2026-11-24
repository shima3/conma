// hof: map, filter, reduce and sort with a comparing function, on an array of 100 numbers, N_ARG times
const n = N_ARG
let acc = 0
for (let r = 0; r < n; r++) {
  const a = []
  for (let i = 0; i < 100; i++) a.push((i * 37 + r) % 101)
  const b = a.map(x => x * 2).filter(x => x % 3 !== 0)
  b.sort((x, y) => x - y)
  acc = acc + b.reduce((s, x) => s + x, 0) + b[0] + b[b.length - 1]
}
console.log(acc)

// json: JSON.stringify and JSON.parse of an object with an array of 20 objects, N_ARG times
const n = N_ARG
let total = 0
for (let r = 0; r < n; r++) {
  const items = []
  for (let i = 0; i < 20; i++) items.push({ id: i + r, name: `item${i}`, tags: ["a", "b"], ok: i % 2 === 0 })
  const text = JSON.stringify({ round: r, items: items })
  const back = JSON.parse(text)
  total = total + text.length + back.items[19].id + (back.items[4].ok ? 1 : 0)
}
console.log(total)

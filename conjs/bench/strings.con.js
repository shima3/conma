// strings: template literals, split, join, slice, indexOf, replaceAll and length, N_ARG times
const n = N_ARG
let total = 0
for (let i = 0; i < n; i++) {
  const line = `item-${i}:alpha,beta,gamma,${i % 7}`
  const parts = line.split(",")
  const head = parts[0].slice(0, parts[0].indexOf(":"))
  const joined = parts.join(";").replaceAll("a", "A")
  total = total + joined.length + head.length + (joined.includes("AlphA") ? 1 : 0)
}
console.log(total)

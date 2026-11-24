// tasks: spawn 10 tasks and join them, N_ARG times (tasks.node.js does the same with async functions and await)
const n = N_ARG
let total = 0
for (let r = 0; r < n; r++) {
  const ts = []
  for (let i = 0; i < 10; i++) ts.push(spawn(() => i * r))
  for (const t of ts) total = total + join(t)
}
console.log(total)

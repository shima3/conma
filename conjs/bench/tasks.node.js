// tasks.con.js in Node.js: an async function for each task, and await for join
async function main() {
  const n = N_ARG
  let total = 0
  for (let r = 0; r < n; r++) {
    const ts = []
    for (let i = 0; i < 10; i++) ts.push((async () => i * r)())
    for (const t of ts) total = total + await t
  }
  console.log(total)
}
main()

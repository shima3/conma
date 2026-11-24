// Other statements at the top level on the host (docs/dev/CONJS-HOST-PLAN.md, stage H6d): an if, a block or a try with a
// loop inside runs on the host like a loop (stages H6a and H6c); statements without a loop stay in ConMa. The log is
// the same on every VM.
const n = 4
let total = 0
if (n > 2) {
  for (let i = 0; i < n; i++) total = total + i
} else {
  total = -1
}
console.log("if", total)

// a block with its own declarations, which do not leave it
let squares = []
{
  const k = 3
  let i = 0
  while (i < k) { squares.push(i * i); i++ }
}
console.log("block", squares)

// a try with a loop that throws (a string: the host does not make errors with new Error), caught, and a finally
let log = []
try {
  for (let i = 0; i < 5; i++) {
    if (i === 3) throw `three ${total}`
    log.push(i)
  }
} catch (e) {
  log.push(e)
} finally {
  log.push("finally")
}
console.log("try", log)

// an if with a loop that waits, while another task changes a variable it reads
let shared = 0
const t = spawn(() => { shared = 7 })
if (shared === 0) {
  for (let i = 0; i < 2; i++) sleep(0)
  console.log("after waiting", shared)
}
join(t)

// a statement without a loop stays in ConMa
if (total > 0) console.log("no loop", total)

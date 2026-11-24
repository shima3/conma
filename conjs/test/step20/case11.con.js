// Loops at the top level that wait or call ConMa back (docs/dev/CONJS-HOST-PLAN.md, stage H6c): at each crossing to the
// VM, the top-level variables the loop wrote go to the VM, and those it uses are read again, so other tasks and
// functions of ConMa see the same values as in ConMa. The log is the same on every VM.
let counter = 0
let seen = []
// another task reads counter whenever the loop waits, and changes shared
let shared = 100
const watcher = spawn(() => {
  for (let i = 0; i < 3; i++) { seen.push(counter); shared = shared + 1; sleep(0) }
})
for (let i = 0; i < 3; i++) {
  counter = counter + 10
  sleep(0)
  console.log("after sleep", i, counter, shared)
}
join(watcher)
console.log("seen", seen, "shared", shared)

// join in a top-level loop
const tasks = [spawn(() => 1), spawn(() => 2), spawn(() => 3)]
let total = 0
for (const t of tasks) total = total + join(t)
console.log("total", total)

// a function of ConMa that reads a top-level variable, called back through map: it sees the value the loop just wrote
let base = 0
const addBase = (x) => x + base
let outs = []
for (let i = 1; i <= 3; i++) {
  base = i * 1000
  outs = outs.concat([i].map(addBase))
}
console.log("outs", outs)

// a function of ConMa that writes a top-level variable, called back: the loop sees the new value after it
let flips = 0
const flip = (x) => { flips = flips + 1; return x }
let got = []
for (let i = 0; i < 3; i++) got = got.concat([flips].map(flip))
console.log("got", got, "flips", flips)

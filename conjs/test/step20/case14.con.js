// for...of over values of the VM, on the host (docs/dev/CONJS-HOST-PLAN.md, section 31; found by M1 in shima3/conma#83):
// a top-level loop over the lines of the standard input, which come through a pipe a little at a time (case14.feed.py),
// so that asking for the next line waits. Each next value is asked of the VM by a request, which may wait (a nested
// call back may not). The log is the same on every VM.
const lines = fs.stdin.readLines()
let seen = []
for (const line of lines) seen.push(line.toUpperCase())
console.log("lines", seen)

// leaving a top-level loop over a generator of ConMa early closes it (its finally runs)
function* nums() {
  try { yield 1; yield 2; yield 3 } finally { console.log("nums closed") }
}
const g1 = nums()
let got = []
for (const x of g1) {
  got.push(x)
  if (x === 2) break
}
console.log("got", got)

// a waiting host function (it sleeps) that loops over a generator of ConMa
function total(g) {
  sleep(0)
  let s = 0
  for (const x of g) s = s + x
  return s
}
console.log("total", total(nums()))

// a top-level for...of over an array with a pattern, stepped through on the host
let sum = 0
for (const [a, b] of [[1, 2], [3, 4]]) sum = sum + a * b
console.log("sum", sum)

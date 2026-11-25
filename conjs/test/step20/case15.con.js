// for...of over values of the VM in an ordinary host function (docs/dev/CONJS-HOST-PLAN.md, section 31; M1's review of
// shima3/conma#84): the next value may wait (the lines of the standard input come through a pipe a little at a time,
// case15.feed.py; a generator of ConMa that sleeps), and an ordinary host function cannot wait, so before any call back
// it gives the call to its ConMa version, which can. The log is the same on every VM.
function count(lines) {
  let n = 0
  for (const l of lines) n = n + l.length
  return n
}
console.log("letters", count(fs.stdin.readLines()))

function* slow() { yield 1; sleep(10); yield 2 }
function sumOf(g) {
  let s = 0
  for (const x of g) s = s + x
  return s
}
console.log("sum", sumOf(slow()))

// an array is still stepped through on the host
function sumArray(a) {
  let s = 0
  for (const x of a) s = s + x
  return s
}
console.log("array", sumArray([4, 5, 6]))

// Loops at the top level on the host (docs/dev/CONJS-HOST-PLAN.md, stage H6a): a loop that the host can write runs
// there as a statement function; the top-level variables declared before it come in as arguments, and those it
// assigns are written back after it. A loop that would go to the VM runs in ConMa instead. The log is the same on
// every VM.
const n = 5
let total = 0
let names = []
const box = { hits: 0 }
for (let i = 0; i < n; i++) {
  total = total + i * i
  names.push(`n${i}`)
  box.hits++
}
console.log(total, names.join(","), box.hits)

// a function of the host called from a top-level loop, and console.log in order with what follows
function sq(x) { return x * x }
let s = 0
for (const x of [1, 2, 3]) {
  console.log("adding", x)
  s = s + sq(x)
}
console.log("sum", s)

// while and do...while; an assignment of a new array to a top-level variable
let k = 0
let evens = []
while (k < 10) { if (k % 2 === 0) evens = evens.concat([k]); k++ }
do { k-- } while (k > 7)
console.log(k, evens)

// a function of ConMa (it uses **, which the host does not take) called in a loop: the loop runs in ConMa
function cube(x) { return x ** 3 }
let c = 0
for (let i = 1; i <= 3; i++) c = c + cube(i)
console.log("cubes", c)

// a loop that would call ConMa back (f is a function of ConMa, given to map): the host gives up before it goes to the
// VM, drops the lines it kept, and ConMa runs the loop from the start, so each line comes out once
const f = (x) => x + 100
let out = []
for (let i = 0; i < 2; i++) {
  console.log("visit", i)
  out = out.concat([i].map(f))
}
console.log("out", out)

// an exception thrown in a top-level loop: uncaught, with its position
let m = 0
for (let i = 0; i < 3; i++) {
  m = m + i
  if (i === 2) throw new Error(`stop at ${m}`)
}
console.log("not reached")

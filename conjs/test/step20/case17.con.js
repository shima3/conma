// Generator functions on the host (stage H8, section 36 of docs/dev/CONJS-HOST-PLAN.md): what crosses between the
// generator and its reader. The log is the same on every VM, and as Node.js prints.

// the same array given twice is the same array; the reader sees a change made after it was given
function* sameTwice() {
  const a = [1]
  yield a
  a.push(2)
  yield a
}
const st = sameTwice()
const first = st.next().value
const second = st.next().value
console.log(first === second, first)

// the generator sees what the reader changed in its argument between two values
function* watch(list) {
  yield list.length
  yield list.length
}
const shared = [1, 2]
const w = watch(shared)
console.log(w.next().value)
shared.push(3)
console.log(w.next().value)

// console.log in the generator and in the reader, in order
function* chatty(n) {
  for (let i = 0; i < n; i++) {
    console.log("make", i)
    yield i
  }
  console.log("made all")
}
for (const x of chatty(2)) console.log("use", x)
function useAll(n) {
  for (const x of chatty(n)) console.log("use in a host function", x)
  return "used"
}
console.log(useAll(2))

// a function of ConMa called back from the generator (a label keeps it in ConMa)
function twice(x) {
  again: for (;;) { break again }
  return x * 2
}
function* doubled(f, n) {
  for (let i = 1; i <= n; i++) yield f(i)
}
console.log([...doubled(twice, 3)])
function sumDoubled(n) {
  let s = 0
  for (const x of doubled(twice, n)) s += x
  return s
}
console.log("sum", sumDoubled(4))

// a string that is not ASCII before the first value: the generator runs as its ConMa version, from the start
function* letters(s) {
  for (let i = 0; i < s.length; i++) yield s.slice(i, i + 1)
}
console.log([...letters("añ😀b")])

// an exception from the body, after a value
function* failing() {
  yield "ok"
  throw new Error("broken")
}
try {
  for (const x of failing()) console.log(x)
} catch (e) {
  console.log("caught", e.message)
}
function readFailing() {
  const got = []
  try {
    for (const x of failing()) got.push(x)
  } catch (e) {
    got.push(e.message)
  }
  return got
}
console.log(readFailing())

// a generator that waits: other tasks run where it sleeps
function* ticking(n) {
  for (let i = 0; i < n; i++) {
    sleep(20)
    yield i
  }
}
const task = spawn(() => { sleep(30); console.log("the other task ran") })
for (const x of ticking(3)) console.log("tick", x)
join(task)
function countTicks(n) {
  let c = 0
  for (const x of ticking(n)) c++
  return c
}
console.log("ticks", countTicks(2))

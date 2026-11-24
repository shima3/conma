// exceptions: throw, catch and finally through a call, with a class of errors, N_ARG times (a third of them throw)
class Bad extends Error {}
function check(i) {
  if (i % 3 === 0) throw new Bad(`bad ${i}`)
  return i
}
const n = N_ARG
let sum = 0
let caught = 0
let last = ""
for (let i = 0; i < n; i++) {
  try {
    sum = sum + check(i)
  } catch (e) {
    if (e instanceof Bad) {
      caught = caught + 1
      last = e.message
    }
  } finally {
    sum = sum + 1
  }
}
console.log(sum, caught, last)

// loop: counts N down to 0 and an accumulator up from 0 (bench/loop/loop.py)
function loop(n) {
  let acc = 0
  while (n !== 0) {
    n -= 1
    acc += 1
  }
  return acc
}
console.log(loop(N_ARG))

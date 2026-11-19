// fib: the naive recursive Fibonacci function (bench/fib/fib.py)
function fib(n) {
  if (n === 0) return 0
  if (n === 1) return 1
  return fib(n - 1) + fib(n - 2)
}
console.log(fib(N_ARG))

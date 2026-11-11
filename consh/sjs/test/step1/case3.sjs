// 変数、スコープ、クロージャ、再帰、while
let x = 1
{ let x = 2; console.log("inner", x) }
console.log("outer", x)
const make = () => { let n = 0; return () => { n += 1; return n } }
const c1 = make()
const c2 = make()
console.log(c1(), c1(), c2(), c1())
const fact = (n) => { if (n <= 1) { return 1 } return n * fact(n - 1) }
console.log(fact(10), fact(20))
const fib = (n) => n < 2 ? n : fib(n - 1) + fib(n - 2)
console.log(fib(15))
let i = 0
let s = 0
while (i < 100) { s += i; i++ }
console.log(s, i)
let j = 3
while (j > 0) j--
console.log(j)
g = 10
const useG = () => g * 2
console.log(useG())
g = 20
console.log(useG())
let a = 5, b = 6
a *= b
b -= 1
console.log(a, b, a++, a, --b, b)

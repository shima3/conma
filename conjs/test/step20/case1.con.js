// Host functions (docs/dev/CONJS-HOST-PLAN.md, stage H3a): functions of values only, which the wasm VM runs on the
// JavaScript of the host, and the other VMs in ConMa. The log is the same on every VM: where the host cannot be sure
// to give what ConJS gives, it throws, and the ConMa version runs instead (sjs/include/sjs_host.se).
function arith(a, b) { return [a + b, a - b, a * b, a / b, a % b].join(" ") }
function add(a, b) { return a + b }
function sub(a, b) { return a - b }
function mul(a, b) { return a * b }
function div(a, b) { return a / b }
function mod(a, b) { return a % b }
function neg(a) { return -a }
function cmp(a, b) { return `${a < b} ${a <= b} ${a > b} ${a >= b} ${a === b} ${a !== b}` }
function logic(a, b) { return (a && b) || !a }
function pick(c, a, b) { return c ? a : b }
function show(x) { return `<${x}>` }
function len(s) { return s.length }

console.log(arith(7, 2), arith(-7, 2), arith(1, 0), arith(-1, 0), arith(0, -5))
console.log(add(0.1, 0.2), sub(5, 5), neg(0), mul(-0, 1), div(0, -1), mod(-0, 3))
console.log(cmp(1, 2), cmp(2, 2), cmp(NaN, 1))
console.log(cmp("a", "b"))
console.log(logic(1, 2), logic(0, 2), logic("", "x"), logic("a", ""), pick(0, "y", "n"), pick("s", 1, 2))
console.log(show(42), show(-0), show(1.5), show(1e21), show(true), show(undefined), show("s"), show(NaN))
console.log(len("hello"), len(""), len("日本語"), len("😀x"))

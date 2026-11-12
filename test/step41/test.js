// Step41 のログ（test.se と同じ式を JavaScript で求める）: node test.js case1|case2|case3
const A = ["0", "1", "-1", "5", "12345", "-12345", "53256", "2147483647", "2147483648", "-2147483648", "-2147483649", "4294967295", "4294967296", "4294967297", "1.9", "-1.9", "0.5", "-0.5", "1e10", "-1e10", "1e300", "-0", "NaN", "Infinity", "-Infinity"]
const B = ["0", "1", "-1", "3", "31", "32", "33", "-33", "53256", "2147483648", "1.5", "NaN"]
const ops = { "&": (a, b) => a & b, "|": (a, b) => a | b, "^": (a, b) => a ^ b, "<<": (a, b) => a << b, ">>": (a, b) => a >> b, ">>>": (a, b) => a >>> b }
const groups = { case1: ["&", "|", "^"], case2: ["<<", ">>", ">>>"] }
const c = process.argv[2]
if (c === "case3") for (const a of A) console.log(`~ ${a}  = ${~Number(a)}`)
else for (const op of groups[c]) for (const a of A) for (const b of B) console.log(`${op} ${a} ${b} = ${ops[op](Number(a), Number(b))}`)

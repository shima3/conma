// Host functions (docs/dev/CONJS-HOST-PLAN.md, stage H3a): functions of values only, which the wasm VM runs on the
// JavaScript of the host, and the other VMs in ConMa. The log is the same on every VM: where the host cannot be sure
// to give what ConJS gives, it throws, and the ConMa version runs instead (sjs/include/sjs_host.se).
function len(s) { return s.length }
function nth(s, i) { return s[i] }
function methods(s) {
  return `${s.charAt(1)}|${s.at(-1)}|${s.indexOf("b")}|${s.includes("cd")}|${s.startsWith("ab")}|${s.endsWith("x")}|${s.slice(1, 3)}|${s.trim()}|${s.toUpperCase()}|${s.toLowerCase()}|${s.repeat(2)}|${s.replace("b", "B")}|${s.replaceAll("b", "_")}`
}
function dollar(s) { return s.replace("a", "$&$&") }
function count(s, c) {
  let n = 0
  for (let i = 0; i < s.length; i++) {
    if (s[i] === c) n++
  }
  return n
}
function caller(s) { return count(s, "a") + len(s) }

console.log(nth("abc", 0), nth("abc", 3), nth("abc", 1.5), nth("日本", 1))
console.log(methods(" abcdabx "))
console.log(methods("äbcdäbx"))
console.log(dollar("banana"))
console.log(count("banana", "a"), count("日本語本", "本"), caller("abracadabra"))

// Host functions (docs/dev/CONJS-HOST-PLAN.md, stage H3a): functions of values only, which the wasm VM runs on the
// JavaScript of the host, and the other VMs in ConMa. The log is the same on every VM: where the host cannot be sure
// to give what ConJS gives, it throws, and the ConMa version runs instead (sjs/include/sjs_host.se).
function add(a, b) { return a + b }
function mul(a, b) { return a * b }
function show(x) { return `<${x}>` }
function len(s) { return s.length }
function loops(n) {
  let t = 0
  let i = 0
  while (true) {
    i += 1
    if (i % 2 === 0) continue
    if (i > n) break
    t += i
  }
  let j = 0
  do { j++ } while (j < 3)
  for (let k = n; k > 0; k -= 3) t -= 1
  return t * 10 + j
}
function words(n) {
  let s = ""
  for (let i = 0; i < n; i++) s = `${s}${i % 3 === 0 ? "fizz" : "x"}`
  return s
}
function deep(n) { return n === 0 ? 0 : 1 + deep(n - 1) }
function nothing(a) { let b = a }
function partial(a, b) { return `${a}-${b}` }
function scoped(n) {
  let x = 1
  { const x = 2; n = n + x }
  return n + x
}
function notHost(a) { console.log("not on the host"); return a }
function usesNotHost(a) { return notHost(a) + 1 }


console.log(loops(10), loops(0), words(7))
console.log(deep(5000))
console.log(nothing(1), partial(1), partial(1, 2, 3))
console.log(scoped(10))
console.log(usesNotHost(1))
console.log(len([1, 2, 3]), show(null), show([1, 2]))
try { console.log(add([1], 2)) } catch (e) { console.log(String(e)) }
try { console.log(add("a", "b")) } catch (e) { console.log(String(e)) }
try { console.log(mul("2", 3)) } catch (e) { console.log(String(e)) }

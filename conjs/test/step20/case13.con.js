// new Error, the built-in errors, typeof and JSON in host functions (docs/dev/CONJS-HOST-PLAN.md, stage H5e). The
// errors are made by the VM (the same name, message and class as in ConMa); JSON uses the engine of the host where it
// gives what ConJS gives, and the VM elsewhere. The log is the same on every VM.
function check(x) {
  if (typeof x !== "number") throw new TypeError(`not a number: ${typeof x}`)
  if (x < 0) throw new RangeError("negative")
  return x * 2
}
function safe(x) {
  try { return check(x) } catch (e) {
    return `${e.name}: ${e.message} ${e instanceof TypeError} ${e instanceof RangeError} ${e instanceof Error}`
  }
}
console.log(safe(2), safe("a"), safe(-1))
function kinds(a) { return a.map((x) => typeof x).join(",") }
console.log(kinds([1, "s", true, undefined, null, [1], { a: 1 }, (y) => y]))

// JSON on the host, and where it differs from ConJS, in the VM
function roundTrip(o) { const t = JSON.stringify(o); const back = JSON.parse(t); return [t, back.list.length, back.name] }
console.log(roundTrip({ name: "é\"q", list: [1, 2, 3], none: undefined, flag: false }))
function inVm(o) { return JSON.stringify(o) }
console.log(inVm({ x: 1.5 }), inVm({ toJSON() { return 1 } }))
function bad(t) { try { return JSON.parse(t) } catch (e) { return `${e.name}: ${e.message}` } }
console.log(bad("{bad"), bad('{"2":"two","a":1}'))

// an error made on the host and thrown out of it: uncaught, with the position of the throw
function fail(n) {
  const e = new Error(`failed with ${n}`)
  throw e
}
fail(JSON.parse("[7]")[0])

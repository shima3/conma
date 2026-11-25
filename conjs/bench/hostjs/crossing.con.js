// Stage H0 of docs/dev/CONJS-HOST-PLAN.md: what a crossing between the VM and the host costs today, measured with
// ConJS on the wasm VM (conjs/conjs crossing.con.js MODE N).
// MODE:
//   empty  an empty loop of N steps (subtract it from the others)
//   call   a call of a small ConJS function in each step: what the VM spends to run a function called back (3.1)
//   add    host.call("test.add", i, 1) in each step: a crossing from the VM to the host and back with numbers
//   echo   host.call("test.echo", system) in each step, with an object like that of nbody_class (5 bodies of
//          7 numbers, about 45 values): a crossing that copies a graph of values both ways (3.2)
const mode = argv[0]
const n = Number(argv[1])
function twice(x) { return x * 2 }
const bodies = []
for (let i = 0; i < 5; i++) bodies.push({ x: i, y: i, z: i, vx: i, vy: i, vz: i, mass: i })
const system = { bodies: bodies, t: 0 }
let acc = 0
if (mode === "empty") { for (let i = 0; i < n; i++) {} }
else if (mode === "call") { for (let i = 0; i < n; i++) acc = acc + twice(i) }
else if (mode === "add") { for (let i = 0; i < n; i++) acc = acc + host.call("test.add", i, 1) }
else if (mode === "echo") { for (let i = 0; i < n; i++) acc = acc + host.call("test.echo", system)[0].bodies.length }
console.log(mode, acc)

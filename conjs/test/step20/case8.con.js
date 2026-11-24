// Waiting host functions (docs/dev/CONJS-HOST-PLAN.md, stage H4c, B3): a function that calls sleep, join
// or nextEvent, or another waiting host function, runs on the host as a generator; at each call that waits,
// the VProc waits, and the other tasks run and see what the function did so far.

// sleep in a loop: the lines come out in turn with those of the other task
function tick(name, n) {
  const seen = [];
  for (let i = 0; i < n; i++) {
    seen.push(`${name}${i}`);
    console.log(`${name} ${i}`);
    sleep(0);
  }
  return seen;
}
const tb = spawn(() => tick("b", 3));
const ta = tick("a", 3);
console.log(ta.join(","), join(tb).join(","));

// another task changes an object while the host function waits; it sees the change after
function fill(box, n) {
  for (let i = 0; i < n; i++) {
    box.items.push(i);
    sleep(0);
  }
  return box.items.length;
}
const box = { items: [] };
spawn(() => { box.items.push("x"); });
console.log(fill(box, 2), box.items.join(","));

// a waiting host function that calls another one
function twice(name) {
  const a = tick(name, 1);
  const b = tick(name, 1);
  return a.concat(b).length;
}
console.log(twice("c"));

// join: a value, and an exception caught on the host
function waitFor(t) {
  const v = join(t);
  return v * 2;
}
console.log(waitFor(spawn(() => 21)));
function safeJoin(t) {
  try {
    return join(t);
  } catch (e) {
    return `caught ${e.message}`;
  }
}
console.log(safeJoin(spawn(() => { throw new Error("boom"); })));

// a function of ConMa that waits, called back from a waiting host function
function relay(f, x) {
  sleep(0);
  return f(x) + 1;
}
console.log(relay((x) => { sleep(0); return x * 10; }, 4));

// a function expression cannot wait on the host (it is not a generator): later stays in ConMa
function later(a) {
  return a.map((x) => { sleep(0); return x + 1; });
}
console.log(later([1, 2]).join(","));

// after waiting (or calling back), what the host cannot check is done by the VM (the review of shima3/conma#76 by M1):
// a string of more than ASCII, reduce without a first value, sort without a comparing function, $ in replace
function splitAfter(s) { sleep(0); return s.split(","); }
function reduceAfter(a) { sleep(0); return a.reduce((x, y) => x + y); }
function sortAfter(a) { sleep(0); return a.slice().sort(); }
function replaceAfter(s) { sleep(0); return s.replace("a", "$&!"); }
function splitAfterCall(s, cb) { cb(); return s.split(","); }
console.log(splitAfter("é,b"), reduceAfter([1, 2, 3]), sortAfter([3, 1, 2]), replaceAfter("abc"));
console.log(splitAfterCall("é,b", () => 0));
// Math and Object.keys of values that are not numbers or objects of the host, and for...of over a number
function othersAfter(x) {
  sleep(0);
  let msg = "";
  try {
    for (const c of 5) msg = c;
  } catch (e) {
    msg = e.message;
  }
  return `${Math.sqrt(x)} ${Math.max(1, x)} ${Object.keys(x).join(",")} ${msg}`;
}
console.log(othersAfter("4"));

// a TypeError of sleep, after waiting: uncaught, with the position of the call
function bad() {
  console.log("before");
  sleep(0);
  sleep("x");
  console.log("not reached");
}
bad();

// The real vm/wasm/host/input.js on a Node.js EventTarget, with a clock moved by hand (docs/dev/PLATFORM-PLAN-ja.md
// 4.6, stage P5c), for the host files of conjs/test/step14 (case*.host.js, CONMA_HOST of vm/wasm/vm-node.js).
//
//   InputMock(script) → { canvas, hostFunctions, hostFeatures }
//
// script[n - 1] says what happens after the n-th call of input.next, in the next task (so that the wait of that
// call has begun): a list of steps, each ["key", name], ["click", x, y] (dispatched on the canvas, as a page would)
// or ["advance", ms] (the clock moves ms milliseconds and the time limits that are due end, earliest first). An empty
// list does nothing; the wait goes on until a later step. A call after the last entry of the script is an Error
// ("input: the test has no more events"), so that a test cannot hang on a wait nobody ends.
const ConmaInput = require("../../../vm/wasm/host/input.js");

function InputMock(script) {
  let now = 0;
  let timers = []; // { at, f, id }
  let nextId = 1;
  const clock = {
    setTimeout: (f, ms) => {
      const id = nextId++;
      timers.push({ at: now + ms, f, id });
      return id;
    },
    clearTimeout: (id) => {
      timers = timers.filter((t) => t.id !== id);
    },
  };
  function advance(ms) {
    now += ms;
    for (;;) {
      const due = timers.filter((t) => t.at <= now).sort((a, b) => a.at - b.at || a.id - b.id);
      if (due.length === 0) return;
      const t = due[0];
      timers = timers.filter((u) => u !== t);
      t.f();
    }
  }
  const canvas = new EventTarget();
  canvas.width = 320;
  canvas.height = 200;
  function dispatch(step) {
    const [kind, ...args] = step;
    if (kind === "advance") return advance(args[0]);
    const e = new Event(kind === "key" ? "keydown" : "pointerdown");
    if (kind === "key") e.key = args[0];
    else [e.clientX, e.clientY] = args;
    canvas.dispatchEvent(e);
  }
  const input = ConmaInput(canvas, clock);
  const next = input.hostFunctions["input.next"];
  let calls = 0;
  input.hostFunctions["input.next"] = (ms) => {
    calls++;
    if (calls > script.length) throw new Error("input: the test has no more events");
    const steps = script[calls - 1];
    const r = next(ms);
    setTimeout(() => steps.forEach(dispatch), 0);
    return r;
  };
  return { canvas, hostFunctions: input.hostFunctions, hostFeatures: input.hostFeatures };
}
module.exports = InputMock;

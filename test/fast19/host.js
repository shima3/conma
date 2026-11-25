// The functions of the host for test/fast19 (CONMA_HOST of vm/wasm/vm-node.js): each calls a global function
// of test.se back with this.callVm (vm/wasm/conma.js; stage H1 of docs/dev/CONJS-HOST-PLAN.md). An error or
// an escape of the call back is caught and given back as a string.
function back(f, ...args) {
  try {
    return f(...args);
  } catch (e) {
    return `${e.escape ? "escape" : "error"}: ${e.message}`;
  }
}
module.exports = {
  hostFeatures: ["callback"],
  hostFunctions: {
    "cb.twice": function (x) { return back(() => this.callVm("twice", x)); },
    "cb.point": function (x, y) { return back(() => this.callVm("point", x, y)); },
    "cb.deeper": function (x) { return back(() => this.callVm("deeper", x)); },
    "cb.broken": function (x) { return back(() => this.callVm("broken", x)); },
    "cb.waits": function (x) { return back(() => this.callVm("waits", x)); },
    "cb.unknown": function (x) { return back(() => this.callVm("no_such_function", x)); },
    "cb.mark": function (x) { return back(() => this.callVm("mark", x)); },
  },
};

// A canvas that records what is drawn, for test/fast16 case2 (CONMA_HOST of vm/wasm/vm-node.js):
// the functions of vm/wasm/host/canvas.js on a 2D context that writes each call to a list, and
// rec.dump, which returns that list. canvas.frame comes at fixed times (16 ms a frame).
const ConmaCanvas = require("../../vm/wasm/host/canvas.js");
const log = [];
const context = new Proxy(
  {},
  {
    get: (_, key) => (...args) => log.push(key + "(" + args.join(", ") + ")"),
    set: (_, key, value) => (log.push(key + " = " + value), true),
  }
);
let frames = 0;
const host = ConmaCanvas({ width: 320, height: 200 }, { context, requestFrame: (f) => setTimeout(() => f(++frames * 16), 0) });
host.hostFunctions["rec.dump"] = () => log.splice(0);
module.exports = host;

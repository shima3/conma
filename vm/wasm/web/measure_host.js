// A canvas that does nothing, for measure.sh (CONMA_HOST of vm/wasm/vm-node.js): the functions of
// canvas.js for __Host_call__, and globalThis.ctx for __JS_call__, on a 2D context whose methods do nothing.
const ConmaCanvas = require("../host/canvas.js");
const context = new Proxy({}, { get: () => () => {}, set: () => true });
globalThis.ctx = context;
module.exports = ConmaCanvas({ width: 320, height: 200 }, { context, requestFrame: (f) => setTimeout(() => f(0), 0) });

// The host of case2.con.js (the sample vm/wasm/web/stars.con.js): the canvas of vm/wasm/host/canvas.js on a context
// that draws nothing, frames at once, and the input of input_mock.js. The keys: start at the title, move a few
// cells, then Escape. The ghosts sleep by the real clock (hundreds of milliseconds), so they do not move first.
const ConmaCanvas = require("../../../vm/wasm/host/canvas.js");
const InputMock = require("./input_mock.js");
const context = new Proxy({}, { get: () => () => {}, set: () => true });
let frames = 0;
const canvas = ConmaCanvas({ width: 480, height: 360 }, { context, requestFrame: (f) => setTimeout(() => f(++frames * 16), 0) });
const input = InputMock([
  [["key", "Enter"]],                       // the title
  [["key", "ArrowRight"]],
  [["key", "ArrowDown"]],
  [["click", 112, 72]],                     // a click to the right of the player
  [["key", "Escape"]],
]);
module.exports = {
  hostFunctions: { ...canvas.hostFunctions, ...input.hostFunctions },
  hostFeatures: [...canvas.hostFeatures, ...input.hostFeatures],
};

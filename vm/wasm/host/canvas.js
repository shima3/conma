// The platform feature "canvas" (docs/dev/PLATFORM-PLAN-ja.md 3.4, stage P3): the functions of the host
// that draw on a canvas of the browser, for the hooks hostFunctions and hostFeatures of vm/wasm/conma.js.
// A ConMa program calls them with __Host_call__ (or __Host_start__ for canvas.frame), most simply
// through include/platform/canvas.se. How a host adds functions in general: docs/HOST-ja.md.
//
//   const host = ConmaCanvas(canvas, options)   // canvas: an HTMLCanvasElement or an OffscreenCanvas
//   ConmaVM.create(module, write, { hostFunctions: host.hostFunctions, hostFeatures: host.hostFeatures })
//
// options.context: the 2D context to draw on (default canvas.getContext("2d")); options.requestFrame:
// the function that calls back at the next frame with its time in milliseconds (default
// requestAnimationFrame). A test on Node.js gives both (test/fast16/host.js records the drawing).
//
// The functions (the arguments are numbers and strings; each returns nothing unless said):
//   canvas.size                        the width and the height of the canvas, a list of 2 numbers
//   canvas.clear                       clear the whole canvas
//   canvas.fill_style css              canvas.stroke_style css   canvas.line_width w   canvas.font css
//   canvas.fill_rect x y w h           canvas.stroke_rect x y w h
//   canvas.begin_path                  canvas.move_to x y        canvas.line_to x y
//   canvas.arc x y r start end         canvas.close_path         canvas.fill    canvas.stroke
//   canvas.fill_text text x y
//   canvas.draw commands               draw a list of commands at once, each a list of the name without
//                                      "canvas." and its arguments, as ("fill_rect" 0 0 10 10)
//   canvas.frame                       wait for the next frame (a Promise): its time in milliseconds
(function (root) {
  "use strict";
  function ConmaCanvas(canvas, options) {
    const opts = options || {};
    const ctx = opts.context || canvas.getContext("2d");
    const requestFrame = opts.requestFrame || ((f) => requestAnimationFrame(f));
    const num = (name, v) => {
      if (typeof v !== "number") throw new Error("canvas." + name + ": expected a number, got " + typeof v);
      return v;
    };
    const text = (name, v) => {
      if (typeof v !== "string") throw new Error("canvas." + name + ": expected a string, got " + typeof v);
      return v;
    };
    // name: [the kinds of the arguments ("n" or "s"), what it does]
    const commands = {
      clear: ["", () => ctx.clearRect(0, 0, canvas.width, canvas.height)],
      fill_style: ["s", (c) => (ctx.fillStyle = c)],
      stroke_style: ["s", (c) => (ctx.strokeStyle = c)],
      line_width: ["n", (w) => (ctx.lineWidth = w)],
      font: ["s", (f) => (ctx.font = f)],
      fill_rect: ["nnnn", (x, y, w, h) => ctx.fillRect(x, y, w, h)],
      stroke_rect: ["nnnn", (x, y, w, h) => ctx.strokeRect(x, y, w, h)],
      begin_path: ["", () => ctx.beginPath()],
      move_to: ["nn", (x, y) => ctx.moveTo(x, y)],
      line_to: ["nn", (x, y) => ctx.lineTo(x, y)],
      arc: ["nnnnn", (x, y, r, a, b) => ctx.arc(x, y, r, a, b)],
      close_path: ["", () => ctx.closePath()],
      fill: ["", () => ctx.fill()],
      stroke: ["", () => ctx.stroke()],
      fill_text: ["snn", (t, x, y) => ctx.fillText(t, x, y)],
    };
    function run(name, args) {
      const c = commands[name];
      if (!c) throw new Error("canvas.draw: unknown command \"" + name + "\"");
      const [kinds, f] = c;
      if (args.length !== kinds.length)
        throw new Error("canvas." + name + ": expected " + kinds.length + " arguments, got " + args.length);
      f(...args.map((a, i) => (kinds[i] === "n" ? num(name, a) : text(name, a))));
    }
    const hostFunctions = {
      "canvas.size": () => [canvas.width, canvas.height],
      "canvas.draw": (list) => {
        if (!Array.isArray(list)) throw new Error("canvas.draw: expected a list of commands");
        for (const cmd of list) {
          if (!Array.isArray(cmd) || typeof cmd[0] !== "string")
            throw new Error("canvas.draw: expected a command, a list of a name and its arguments");
          run(cmd[0], cmd.slice(1));
        }
        return null;
      },
      "canvas.frame": () => new Promise((resolve) => requestFrame(resolve)),
    };
    for (const name of Object.keys(commands)) {
      hostFunctions["canvas." + name] = (...args) => {
        run(name, args);
        return null;
      };
    }
    return { hostFunctions, hostFeatures: ["canvas"] };
  }
  if (typeof module === "object" && module.exports) module.exports = ConmaCanvas;
  else root.ConmaCanvas = ConmaCanvas;
})(typeof globalThis !== "undefined" ? globalThis : this);

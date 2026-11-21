// The platform feature "input" (docs/dev/PLATFORM-PLAN-ja.md 4.6, stage P5c): the functions of the host
// that let a ConMa program wait for the keys and the clicks of a page, for the hooks hostFunctions and
// hostFeatures of vm/wasm/conma.js. ConJS calls them through nextEvent(ms) and pollEvent()
// (conjs/include/conjs_lib.se).
//
//   const input = ConmaInput(canvas, options)   // canvas: an EventTarget, an HTMLCanvasElement on a page
//   ConmaVM.create(module, write, { hostFunctions: {...canvasHost.hostFunctions, ...input.hostFunctions},
//                                   hostFeatures: [...canvasHost.hostFeatures, ...input.hostFeatures] })
//
// options (all may be left out):
//   keyTarget                   what receives keydown (default canvas; document for keys anywhere on the page)
//   setTimeout, clearTimeout    the clock of the time limits (default those of globalThis; a test gives a
//                               clock it moves by hand, conjs/test/step14/case1.host.js)
//
// The host keeps the keydown events of keyTarget and the pointerdown events of canvas in a queue, in the
// order they come, also when nobody waits. It reads only key, clientX and clientY of an event, so a test
// may dispatch plain Events on a Node.js EventTarget. The arrow keys, the space and Escape do not scroll
// the page (preventDefault).
//
// An event is a list: ("key" name), name being KeyboardEvent.key ("ArrowUp", "a", " ", "Escape"; each
// repeat of a held key is one event), or ("click" x y) in the coordinates of the canvas (clientX and
// clientY less the top left of canvas.getBoundingClientRect(), scaled by the width and the height of the
// canvas over its size on the page; a canvas without getBoundingClientRect takes clientX and clientY).
//
// The functions:
//   input.next ms   wait for the first event of the queue (a Promise): the event, or null when ms is 0 or
//                   more and no event comes in ms milliseconds. A negative ms does not limit the time.
//                   Waits are served in the order they began, and a wait that timed out leaves the queue
//                   of waits, so it takes no later event.
//   input.poll      the first event of the queue, or null when it is empty (it does not wait)
(function (root) {
  "use strict";
  const QUIET_KEYS = new Set(["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", " ", "Escape"]);

  function ConmaInput(canvas, options) {
    const opts = options || {};
    const keyTarget = opts.keyTarget || canvas;
    const setTimer = opts.setTimeout || ((f, ms) => globalThis.setTimeout(f, ms));
    const clearTimer = opts.clearTimeout || ((t) => globalThis.clearTimeout(t));
    const events = []; // events nobody has taken
    const waits = []; // { resolve, timer } in the order they began

    function put(ev) {
      const w = waits.shift();
      if (w) {
        if (w.timer !== undefined) clearTimer(w.timer);
        w.resolve(ev);
      } else {
        events.push(ev);
      }
    }
    function point(e) {
      let x = e.clientX, y = e.clientY;
      if (typeof canvas.getBoundingClientRect === "function") {
        const r = canvas.getBoundingClientRect();
        x -= r.left;
        y -= r.top;
        if (r.width > 0 && r.height > 0 && canvas.width && canvas.height) {
          x *= canvas.width / r.width;
          y *= canvas.height / r.height;
        }
      }
      return [Math.round(x), Math.round(y)];
    }

    keyTarget.addEventListener("keydown", (e) => {
      if (typeof e.key !== "string") return;
      if (QUIET_KEYS.has(e.key) && typeof e.preventDefault === "function") e.preventDefault();
      put(["key", e.key]);
    });
    canvas.addEventListener("pointerdown", (e) => {
      if (typeof canvas.focus === "function") canvas.focus();
      put(["click", ...point(e)]);
    });

    const hostFunctions = {
      "input.next": (ms) => {
        if (typeof ms !== "number" || Number.isNaN(ms)) throw new Error("input.next: expected milliseconds (a Number)");
        if (events.length > 0) return events.shift();
        return new Promise((resolve) => {
          const w = { resolve, timer: undefined };
          waits.push(w);
          if (ms >= 0) {
            w.timer = setTimer(() => {
              const i = waits.indexOf(w);
              if (i >= 0) waits.splice(i, 1);
              resolve(null);
            }, ms);
          }
        });
      },
      "input.poll": () => (events.length > 0 ? events.shift() : null),
    };
    return { hostFunctions, hostFeatures: ["input"] };
  }
  if (typeof module === "object" && module.exports) module.exports = ConmaInput;
  else root.ConmaInput = ConmaInput;
})(typeof globalThis !== "undefined" ? globalThis : this);

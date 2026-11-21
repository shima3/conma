// The functions of the host for conjs/test/step5 case4 (CONMA_HOST of vm/wasm/vm-node.js).
module.exports = {
  hostFeatures: ["demo"],
  hostFunctions: {
    "demo.sum": (a, b) => a + b,
    "demo.slow_sum": (a, b) => new Promise((resolve) => setTimeout(() => resolve(a + b), 10)),
    "demo.reject": () => Promise.reject(new Error("demo.reject: no")),
    "demo.throw": () => {
      throw new Error("demo.throw: boom");
    },
    "demo.slow_bool": () => Promise.resolve(true),
    "demo.slow_object": () => Promise.resolve({ a: [1, false] }),
    "demo.slow_date": () => Promise.resolve(new Date(0)),
  },
};

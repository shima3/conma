// The functions of the host for test/fast14 case2 (CONMA_HOST of vm/wasm/vm-node.js).
module.exports = {
  hostFeatures: ["demo"],
  hostFunctions: {
    "demo.sum3": (a, b, c) => a + b + c,
    "demo.echo": (...args) => args,
    "demo.types": (...args) => args.map((v) => (Array.isArray(v) ? "array" : Object.is(v, -0) ? "-0" : typeof v)),
    "demo.throw": () => {
      throw new Error("demo.throw: boom");
    },
    "demo.bool": () => true,
    "demo.object": () => ({ a: 1, b: [true, "s"], c: {} }),
    "demo.date": () => new Date(0),
    "demo.promise": () => Promise.resolve(1),
  },
};

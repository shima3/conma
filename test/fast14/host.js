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
    "demo.promise": () => Promise.resolve(1),
  },
};

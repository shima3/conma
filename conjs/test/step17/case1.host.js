// conjs/test/step17 case1 のホストの関数（wasm 版の VM の CONMA_HOST）：決まらない Promise を返す
module.exports = { hostFeatures: ["demo"], hostFunctions: { "demo.never": () => new Promise(() => {}) } };

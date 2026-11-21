// conjs/examples/host_demo.con.js のホストの関数を、JavaScript で書いたもの（docs/HOST-ja.md、段階 P7）。
// wasm 版の VM を Node.js で動かすとき、CONMA_HOST に、このファイルを渡す：
//   CONMA_VM=wasm CONMA_HOST=conjs/examples/host_demo.host.js bin/lang conjs conjs/examples/host_demo.con.js
// ブラウザでは、同じ hostFunctions と hostFeatures を ConMaVM.create の hooks に渡す（vm/wasm/conma.js）。
// Rust で同じものを書いたのが vm/examples/host_demo.rs。
module.exports = {
  hostFeatures: ["demo"],
  hostFunctions: {
    // すぐ返る関数：値を返すか、例外を投げる
    "demo.add": (...args) => {
      if (!args.every((a) => typeof a === "number")) throw new Error("demo.add: expected Numbers");
      return args.reduce((s, a) => s + a, 0);
    },
    "demo.info": (name, tags) => {
      if (typeof name !== "string" || !Array.isArray(tags)) throw new Error("demo.info: expected a name and a list of tags");
      return { name, count: tags.length, short: [...name].length < 4 };
    },
    // 待つ関数：Promise を返す（host.wait で呼ぶ。host.call で呼ぶとエラー）
    "demo.delay": (ms, text) => {
      if (typeof ms !== "number" || ms < 0 || typeof text !== "string")
        return Promise.reject(new Error("demo.delay: expected milliseconds and a text"));
      return new Promise((resolve) => setTimeout(() => resolve(text), ms));
    },
    // Rust の Done を捨てたときと同じエラー
    "demo.drop": () => Promise.reject(new Error("demo.drop: the host dropped the call")),
  },
};

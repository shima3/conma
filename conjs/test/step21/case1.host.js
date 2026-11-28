// async/await で書いた JavaScript を、ホストの待つ関数にする（docs/HOST-ja.md の 3.2 節の例。段階 P10）。
// async 関数は Promise を返すので、そのまま待つ関数になる。ConJS からは host.wait で呼び、呼んだ処理だけが待つ
const fs = require("node:fs/promises");
const path = require("node:path");
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

module.exports = {
  hostFunctions: {
    // ファイルを読んで、空でない行を数える
    "files.countLines": async (name) => {
      const text = await fs.readFile(path.join(__dirname, name), "utf8");
      return text.split("\n").filter((line) => line !== "").length;
    },
    // ms ミリ秒待ってから text を返す
    "files.later": async (ms, text) => {
      await delay(ms);
      return text;
    },
  },
  hostFeatures: ["files"],
};

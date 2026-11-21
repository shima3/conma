// counter_server.node.js（Node.js で書いたもの）を demo で動かし、counter_server.con.js（case2。wasm 版の VM）と同じ
// 出力になることを確かめる（docs/dev/PLATFORM-PLAN-ja.md の 4.10 節、段階 P6b）。違えば両方を出し、終了ステータス 1。
// サーバは compare.sh の形（同じ引数で動かして出力を比べる）で比べられるが、ConJS の側は機能 "server" のある wasm 版の
// VM でしか動かないので、case2.wasm.log（ConJS の出力）と比べる。conjs/test/compare.sh が動かす。
//   node conjs/test/step18/case2.node-check.js
"use strict";
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const sample = path.join(__dirname, "..", "..", "examples", "counter_server.node.js");
const work = fs.mkdtempSync(path.join(os.tmpdir(), "conjs-step18-"));
let out;
try {
  out = execFileSync("node", [sample, "demo"], { cwd: work, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
} finally {
  fs.rmSync(work, { recursive: true, force: true });
}
const log = fs.readFileSync(path.join(__dirname, "case2.wasm.log"), "utf8");
const expected = log.slice(0, log.lastIndexOf("exit: "));
if (out !== expected) {
  console.log("== step18: counter_server.node.js demo differs from case2.wasm.log");
  console.log("-- Node.js:");
  process.stdout.write(out);
  console.log("-- ConJS:");
  process.stdout.write(expected);
  process.exit(1);
}

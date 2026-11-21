// counter_server.con.js と同じことを Node.js で書いたもの（docs/dev/PLATFORM-PLAN-ja.md の 4.10 節、段階 P6b）。
// http.createServer は、要求ごとにコールバックを呼ぶ。コールバックの中で待つ（setTimeout、ファイルを読む）には async と
// await が要り、要求を受け取るループは Node.js の中にある。ファイルの行は、readline の Interface から for await で受け取る。
//   node conjs/examples/counter_server.node.js [port [ディレクトリ]]
//   node conjs/examples/counter_server.node.js demo
// conjs/test の step18 が、demo で counter_server.con.js と同じ出力になることを確かめる（case2.node-check.js）
"use strict";
const http = require("http");
const fs = require("fs");
const readline = require("readline");
const { setTimeout: sleep } = require("timers/promises");

let count = 0;

async function lineCount(dir, name) {
  const file = await fs.promises.open(`${dir}/${name}`);
  try {
    let n = 0;
    const rl = readline.createInterface({ input: file.createReadStream(), crlfDelay: Infinity });
    for await (const line of rl) n++;
    return n;
  } finally {
    await file.close();
  }
}

async function handle(server, dir, req, res) {
  const respond = (status, headers, body) => {
    if (body === undefined) [headers, body] = [{}, headers];
    res.writeHead(status, headers);
    res.end(body);
  };
  const q = req.url.indexOf("?");
  const path = q < 0 ? req.url : req.url.slice(0, q);
  const query = q < 0 ? "" : req.url.slice(q + 1);
  if (path === "/count") {
    count = count + 1;
    respond(200, `${count}\n`);
  } else if (path === "/slow") {
    await sleep(200);
    respond(200, "slow\n");
  } else if (path === "/file") {
    const name = query.startsWith("name=") ? query.slice(5) : "";
    if (name === "" || name.includes("/") || name.includes("..")) {
      respond(400, "bad name\n");
      return;
    }
    let n;
    try {
      n = await lineCount(dir, name);
    } catch (e) {
      respond(404, "no such file\n");
      return;
    }
    respond(200, { "content-type": "text/plain" }, `${n} lines\n`);
  } else if (path === "/stop") {
    respond(200, "bye\n");
    server.close();
  } else {
    respond(404, "not found\n");
  }
}

function listen(port, dir) {
  const server = http.createServer((req, res) => {
    // 本体は使わないが、終わりまで読む
    req.resume();
    req.on("end", () => handle(server, dir, req, res));
  });
  return new Promise((resolve) => server.listen(port, "127.0.0.1", () => resolve(server)));
}

async function demo(base) {
  const get = async (path) => {
    const r = await fetch(`${base}${path}`);
    return `${path} -> ${r.status} ${(await r.text()).trim()}`;
  };
  await fs.promises.writeFile("counter_demo.txt", "one\ntwo\nthree\n");
  console.log(await get("/count"));
  const order = [];
  const slow = get("/slow").then((line) => order.push(line));
  await sleep(50);
  order.push(await get("/count"));
  await slow;
  for (const line of order) console.log(line);
  console.log(await get("/file?name=counter_demo.txt"));
  console.log(await get("/file?name=../counter_demo.txt"));
  console.log(await get("/file?name=missing.txt"));
  console.log(await get("/nothing"));
  console.log(await get("/stop"));
}

async function main() {
  const args = process.argv.slice(2);
  if (args[0] === "demo") {
    const server = await listen(0, ".");
    const closed = new Promise((resolve) => server.on("close", resolve));
    await demo(`http://127.0.0.1:${server.address().port}`);
    await closed;
    console.log("stopped");
  } else {
    const server = await listen(args.length > 0 ? Number(args[0]) : 8000, args.length > 1 ? args[1] : ".");
    console.log(`listening on http://127.0.0.1:${server.address().port}/`);
    server.on("close", () => console.log("stopped"));
  }
}

main();

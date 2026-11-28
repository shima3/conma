// 小さな HTTP のサーバ（docs/dev/PLATFORM-PLAN-ja.md の 4.10 節、段階 P6b。Node.js だけ）。要求をループで
// 1 つずつ受け取り、要求ごとに spawn した処理が、上から下に答える。
//   /count            数を 1 つ増やして答える
//   /slow             200 ミリ秒待ってから答える（その間も、ほかの要求に答える）
//   /file?name=名前   答えるディレクトリの中のファイルの行の数を答える（「/」か「..」を含む名前は 400、ないファイルは 404）
//   /stop             サーバを閉じる（受け取った要求には答え、プログラムが終わる）
// 同じことを Node.js の http.createServer で書いたものが counter_server.node.js。
//   conjs/conjs conjs/examples/counter_server.con.js [port [ディレクトリ]]   127.0.0.1 で答える（既定は 8000 と「.」）
//   conjs/conjs conjs/examples/counter_server.con.js demo   OS が選ぶ port で答え、自分に要求を送って答えを出し、止める
//                                                  （今のディレクトリに counter_demo.txt を書く）
let count = 0;

function lineCount(dir, name) {
  let n = 0;
  for (const line of fs.readLines(`${dir}/${name}`)) n++;
  return n;
}

function handle(server, dir, req) {
  const q = req.url.indexOf("?");
  const path = q < 0 ? req.url : req.url.slice(0, q);
  const query = q < 0 ? "" : req.url.slice(q + 1);
  if (path === "/count") {
    count = count + 1; // 読んでから書くまでの間に、ほかの処理は動かない（待つ所がない）
    req.respond(200, `${count}\n`);
  } else if (path === "/slow") {
    sleep(200);
    req.respond(200, "slow\n");
  } else if (path === "/file") {
    const name = query.startsWith("name=") ? query.slice(5) : "";
    if (name === "" || name.includes("/") || name.includes("..")) {
      req.respond(400, "bad name\n");
      return;
    }
    let n;
    try {
      n = lineCount(dir, name);
    } catch (e) {
      req.respond(404, "no such file\n");
      return;
    }
    req.respond(200, { "content-type": "text/plain" }, `${n} lines\n`);
  } else if (path === "/stop") {
    req.respond(200, "bye\n");
    server.close();
  } else {
    req.respond(404, "not found\n");
  }
}

// サーバが閉じられるまで、要求を受け取る
function serve(server, dir) {
  for (;;) {
    const req = server.accept();
    if (req === undefined) return;
    spawn(() => handle(server, dir, req));
  }
}

// サーバに要求を送り、答えを出す
function demo(base) {
  const get = (path) => {
    const r = fetch(`${base}${path}`);
    return `${path} -> ${r.status} ${r.text().trim()}`;
  };
  fs.writeFile("counter_demo.txt", "one\ntwo\nthree\n");
  console.log(get("/count"));
  // /slow が待つ間に、後から送った /count が先に答える
  const order = [];
  const slow = spawn(() => order.push(get("/slow")));
  sleep(50);
  order.push(get("/count"));
  join(slow);
  for (const line of order) console.log(line);
  console.log(get("/file?name=counter_demo.txt"));
  console.log(get("/file?name=../counter_demo.txt"));
  console.log(get("/file?name=missing.txt"));
  console.log(get("/nothing"));
  console.log(get("/stop"));
}

if (argv[0] === "demo") {
  const server = http.listen(0);
  const t = spawn(() => serve(server, "."));
  demo(`http://127.0.0.1:${server.port}`);
  join(t);
  console.log("stopped");
} else {
  const server = http.listen(argv.length > 0 ? Number(argv[0]) : 8000);
  console.log(`listening on http://127.0.0.1:${server.port}/`);
  serve(server, argv.length > 1 ? argv[1] : ".");
  console.log("stopped");
}

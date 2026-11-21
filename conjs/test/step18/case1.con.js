// http.listen と、サーバの accept()、respond()、close()（docs/dev/PLATFORM-PLAN-ja.md の 4.10 節、段階 P6b）。
// 機能 "server" は、wasm 版の VM を Node.js で動かすときだけある（ほかの VM のログは、エラーの 1 行）
const server = http.listen(0);
console.log("port:", typeof server.port, server.port > 0);
const base = `http://127.0.0.1:${server.port}`;

// GET と POST、ヘッダ、本体、JSON
const get = spawn(() => {
  const r = fetch(`${base}/hello?x=1`, { headers: { "X-Test": "hi" } });
  return `${r.status} ${r.text()}`;
});
const req1 = server.accept();
console.log(req1.method, req1.url, req1.headers["x-test"], JSON.stringify(req1.body));
console.log(Object.keys(req1));
req1.respond(200, { "content-type": "text/plain" }, "hello");
try {
  req1.respond(200, "again");
} catch (e) {
  console.log("second respond:", e.message);
}
console.log("GET:", join(get));

// 不正なヘッダは Error で、要求は残り、答え直せる（M1 の PR shima3/conma#28 のレビューの 1）
const bad = spawn(() => {
  const r = fetch(`${base}/bad`);
  return `${r.status} ${r.text()}`;
});
const reqBad = server.accept();
try {
  reqBad.respond(200, { "x-bad": "a\nb" }, "no");
} catch (e) {
  console.log("bad header:", e.message);
}
try {
  reqBad.respond(200, { "x-nested": { a: 1 } }, "no");
} catch (e) {
  console.log("nested header:", e.message);
}
reqBad.respond(200, { "x-good": "ok" }, "answered again");
console.log("bad:", join(bad));

const post = spawn(() => {
  const r = fetch(`${base}/json`, { method: "POST", body: '{"a":[1,"ü"]}' });
  return `${r.status} ${r.text()}`;
});
const req2 = server.accept();
console.log(req2.method, req2.url, req2.json().a[1], req2.body.length);
req2.respond(201, "made");
console.log("POST:", join(post));

// 本体が 1 MB を超える要求は、受け取らずに 413（1 MB の文字列は、倍にしていって作る。repeat は長い文字列で遅い）
let mb = "x";
for (let i = 0; i < 20; i++) mb = `${mb}${mb}`;
const big = fetch(`${base}/big`, { method: "POST", body: `${mb}x` });
console.log("big:", big.status);
const ok = spawn(() => fetch(`${base}/after`, { method: "POST", body: mb }).status);
const req3 = server.accept();
console.log(req3.url, req3.body.length);
req3.respond(204);
console.log("1 MB:", join(ok));

// 使われている port
try {
  http.listen(server.port);
} catch (e) {
  console.log("listen:", e.message);
}

// 受け取った要求には、close() の後でも答えられる。まだ受け取っていない要求は 503
const slow = spawn(() => fetch(`${base}/slow`).text());
const req4 = server.accept();
const queued = spawn(() => fetch(`${base}/queued`).status);
sleep(100);
server.close();
sleep(50);
req4.respond(200, "slow done");
console.log("slow:", join(slow));
console.log("queued:", join(queued));
console.log("accept after close:", server.accept());

// 待っている accept は、close() で undefined を受け取る
const s2 = http.listen(0);
const waiting = spawn(() => s2.accept());
sleep(20);
s2.close();
console.log("waiting accept:", join(waiting));

// 引数の誤り
try {
  http.listen("80");
} catch (e) {
  console.log(e.name, e.message);
}
try {
  http.listen(70000);
} catch (e) {
  console.log(e.message);
}
console.log("done");

// fetch の options：method、headers、body。HEAD は本体のない応答（curl の道でも成功する）
const base = `http://127.0.0.1:${fs.readFile("port.txt")}`;
const p = fetch(`${base}/echo`, { method: "POST", body: "the body", headers: { "X-Test": "yes" } });
console.log(p.status, p.text());
const q = fetch(`${base}/echo`, { method: "PUT", body: "put body", headers: { "X-Test": 42 } });
console.log(q.status, q.text());
const h = fetch(`${base}/hello`, { method: "HEAD" });
console.log("HEAD", h.status, h.ok, JSON.stringify(h.text()));
// 2 つの処理が並べて待つ
const t1 = spawn(() => fetch(`${base}/hello`).text());
const t2 = spawn(() => fetch(`${base}/json`).status);
console.log(join(t1), join(t2));

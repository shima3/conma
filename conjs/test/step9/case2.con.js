// fetch の options：method、headers、body
const base = `http://127.0.0.1:${fs.readFile("port.txt")}`;
const p = fetch(`${base}/echo`, { method: "POST", body: "the body", headers: { "X-Test": "yes" } });
console.log(p.status, p.text());
const q = fetch(`${base}/echo`, { method: "PUT", body: "put body", headers: { "X-Test": 42 } });
console.log(q.status, q.text());
// 2 つの処理が並べて待つ
const t1 = spawn(() => fetch(`${base}/hello`).text());
const t2 = spawn(() => fetch(`${base}/json`).status);
console.log(join(t1), join(t2));

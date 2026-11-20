// fetch：GET の状態と本体、json()、404（例外を投げず ok が偽）。サーバは server.py（run_case.sh が立て、ポートを port.txt に書く）
const base = `http://127.0.0.1:${fs.readFile("port.txt")}`;
const r = fetch(`${base}/hello`);
console.log(r.status, r.ok, r.text());
const j = fetch(`${base}/json`).json();
console.log(JSON.stringify(j), j.a[1], typeof j.b);
const n = fetch(`${base}/nope`);
console.log(n.status, n.ok, n.text());

// 自分を含むオブジェクトと配列は、ホストに渡せない（たどり終わらないので TypeError。M2 の PR shima3/conma#26 のレビューの 1）。
// 同じオブジェクトを 2 か所に持つだけなら、渡せる
const o = { a: 1 }; o.self = o;
try { host.call("test.echo", o); } catch (e) { console.log(String(e)); }
const arr = [1]; arr.push(arr);
try { host.wait("test.later", 0, arr); } catch (e) { console.log(String(e)); }
const shared = { s: 1 };
console.log(host.call("test.echo", { p: shared, q: shared }, [shared, shared]));

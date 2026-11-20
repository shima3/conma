// オブジェクトのキーとメンバーの名前には、async、await、Promise を使える（M2 の PR shima3/conma#4 のレビューの 1）
const o = { async: 1, await: 2, Promise: 3, "x": 4 };
o.Promise = o.async + o.await;
const { async: a, Promise: p } = o;
console.log(o, a, p, JSON.stringify({ await: [1] }));

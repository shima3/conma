// 締め切りが同じなら呼んだ順、違えば締め切りの順に届く。届いた処理は列の最後に入る
const names = ["p", "q", "r", "s"];
const counts = [3, 1, 3, 1];
const tasks = names.map((n, i) => spawn(() => {
  const r = host.wait("test.later", counts[i], n);
  console.log("got", r[1]);
  return n;
}));
console.log("spawned", tasks.length);
console.log(tasks.map(t => join(t)).join(" "));

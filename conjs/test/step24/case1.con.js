// Queue（docs/dev/CONJS-SHELL-PLAN.md の 7 節、段階 S1）：ホストの、待つキュー。親を持て、読み手は親の要素の後に、自分の要素を読む
const q = new Queue();
console.log(typeof q, q.closed);
q.push(1);
q.push("two");
const o = { k: 3 };
q.push(o);
q.close();
console.log(q.closed);
const got = [];
for (const x of q) got.push(x);
console.log(got[0], got[1], got[2] === o);
try {
  q.push(4);
} catch (e) {
  console.log(e.message);
}
try {
  Queue();
} catch (e) {
  console.log(e.message);
}
try {
  new Queue(1);
} catch (e) {
  console.log(e.message);
}
// 親：子を読むと、親が閉じて空になってから、子の要素が来る（子に先に入れても）
const a = new Queue();
const b = new Queue(a);
b.push("b1");
a.push("a1");
const t = spawn(() => {
  sleep(10);
  a.push("a2");
  a.close();
  b.close();
});
for (const x of b) console.log("b:", x);
join(t);
// 読んだ要素は消える：同じ親の 2 つの子は、親の要素を分け合う
const p = new Queue();
const c1 = new Queue(p);
const c2 = new Queue(p);
p.push("p1");
p.push("p2");
p.close();
c1.push("c1");
c1.close();
c2.close();
console.log([...c1].join(","), [...c2].join(","));
// 2 つの処理が同じキューを読み、1 つの処理が書く
const w = new Queue();
const r1 = spawn(() => {
  let n = 0;
  for (const x of w) n += x;
  return n;
});
const r2 = spawn(() => {
  let n = 0;
  for (const x of w) n += x;
  return n;
});
for (let i = 1; i <= 10; i++) {
  w.push(i);
  if (i % 3 === 0) sleep(1);
}
w.close();
console.log(join(r1) + join(r2));
// break で抜けても、残りは読める
const s = new Queue();
s.push(1);
s.push(2);
s.push(3);
s.close();
for (const x of s) {
  console.log("first", x);
  break;
}
for (const x of s) console.log("rest", x);

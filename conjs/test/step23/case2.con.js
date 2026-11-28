// ホストの関数（docs/dev/CONJS-HOST-PLAN.md）の中でも、ホストのオブジェクトは同じオブジェクト（段階 S0b。case2.host.js）
function bump(b, k) {
  let s = 0;
  for (let i = 0; i < k; i++) s += b.inc();
  return s;
}
function addAll(b, xs) {
  for (const x of xs) b.add(x);
  return b.items.length;
}
function sumItems(b) {
  let s = 0;
  for (const x of b) s += x;
  return s;
}
const b = host.wait("test.box", 1);
console.log(bump(b, 3), b.n);
console.log(addAll(b, [1, 2, 3]), sumItems(b));
const same = (x, y) => x === y;
console.log(same(b, b));

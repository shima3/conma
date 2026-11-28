// ホストのオブジェクトを、参照で持って使う（docs/dev/CONJS-SHELL-PLAN.md の段階 S0b。case1.host.js）。
// メンバーを読み書きし、メソッドを呼び、for...of で回す。渡した ConJS のオブジェクトは、写されずに同じものとして戻る
const b = host.wait("test.box", 1);
console.log(typeof b, b.n, b.inc(), b.inc(), b.n);
b.n = 10;
console.log(b.n);
const o = { k: 1 };
console.log(b.add(o), b.add("s"));
console.log(b.item(0) === o, b.item(1));
o.k = 2;
console.log(b.item(0).k);
for (const x of b) console.log(x);
console.log(b === b, b === host.wait("test.box", 1));
const f = b.inc;
console.log(typeof f, f());
console.log(b.info());
try {
  b.fail();
} catch (e) {
  console.log(e.message);
}
try {
  b.nothing();
} catch (e) {
  console.log(e.message);
}
// Promise を返すメソッドは、この処理だけが待つ
const t1 = spawn(() => console.log("meanwhile"));
console.log(b.later(20));
join(t1);
console.log(b, `${b}`);
// 別の処理が埋める列を、for...of で、値が来るのを待ちながら回す
const c = host.wait("test.chan");
const t2 = spawn(() => {
  for (let i = 0; i < 3; i++) {
    sleep(10);
    c.push({ i });
  }
  c.close();
});
for (const x of c) console.log("got", x.i);
join(t2);

// spawn、join、host とライブラリを使わない ConJS のプログラム（JavaScript のプログラムでもある）。
// ../compare.sh が Node.js と出力を比べる：関数、クロージャ、再帰、配列とオブジェクト
function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
console.log([0, 1, 2, 10, 20].map(fib));
const makeCounter = () => { let n = 0; return () => ++n; };
const c = makeCounter();
c(); c();
console.log("counter:", c());
const o = { a: 1, b: [2, 3], c: { d: "x" } };
console.log(JSON.stringify(o), Object.keys(o));
const { a, b: [, three], ...rest } = o;
console.log(a, three, JSON.stringify(rest));
console.log([3, 1, 2].join(","), [1, 2, 3].reduce((s, x) => s + x, 0));
console.log(`template ${a + three} ${typeof o}`);

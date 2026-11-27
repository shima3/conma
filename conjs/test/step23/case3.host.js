// ホストのオブジェクトを参照で ConJS に渡す（docs/dev/CONJS-SHELL-PLAN.md の段階 S0b。case1.con.js）。
// Symbol.for("conma.byRef") の印を付けたクラスのオブジェクトは、写されずに、同じオブジェクトとして行き来する
const BYREF = Symbol.for("conma.byRef");

class Box {
  constructor(n) {
    this.n = n;
    this.items = [];
  }
  inc() {
    return ++this.n;
  }
  add(x) {
    this.items.push(x);
    return this.items.length;
  }
  item(i) {
    return this.items[i];
  }
  later(ms) {
    return new Promise((resolve) => setTimeout(() => resolve(this.n), ms));
  }
  info() {
    return { n: this.n, list: [1, 2] };
  }
  fail() {
    throw new Error("boom");
  }
  *[Symbol.iterator]() {
    yield* this.items;
  }
}
Box.prototype[BYREF] = true;

// 読み手が、値が来るまで待つ列（next が Promise を返す）
class Chan {
  constructor() {
    this.q = [];
    this.waiters = [];
    this.closed = false;
  }
  push(x) {
    const w = this.waiters.shift();
    if (w) w({ done: false, value: x });
    else this.q.push(x);
  }
  close() {
    this.closed = true;
    for (const w of this.waiters) w({ done: true });
    this.waiters = [];
  }
  [Symbol.iterator]() {
    return {
      next: () => {
        if (this.q.length > 0) return { done: false, value: this.q.shift() };
        if (this.closed) return { done: true };
        return new Promise((resolve) => this.waiters.push(resolve));
      },
    };
  }
}
Chan.prototype[BYREF] = true;

module.exports = {
  hostFunctions: {
    "test.box": (n) => new Box(n),
    "test.chan": () => new Chan(),
    // 渡されたホストのオブジェクトを使う（case3）
    "test.use": (b) => (b instanceof Box ? `got box ${b.inc()}` : `not a box: ${typeof b}`),
    "test.useAll": (o) => o.list.map((b) => b.n).join(","),
  },
};

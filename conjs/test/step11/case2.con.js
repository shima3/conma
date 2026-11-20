// クラス式、名前のあるクラス式、3 段の継承、static の this、ジェネレータのメソッド、自分のプロパティが先
const Anon = class {
  hello() { return "hi"; }
};
console.log(Anon, new Anon().hello());
const Fact = class F {
  static of(n) { return n <= 1 ? 1 : n * F.of(n - 1); }
};
console.log(Fact.of(5));

class A {
  constructor() { this.trail = ["A"]; }
  who() { return "A"; }
  static make() { return new this(); }
}
class B extends A {
  constructor() { super(); this.trail.push("B"); }
  who() { return `B<${super.who()}`; }
}
class C extends B {
  constructor() { super(); this.trail.push("C"); }
  who() { return `C<${super.who()}`; }
}
const c = C.make();
console.log(c.trail, c.who(), c instanceof A, c instanceof B, c instanceof C);
console.log(B.make() instanceof C, A.make().who());

class Range {
  constructor(lo, hi) { this.lo = lo; this.hi = hi; }
  *values() {
    for (let i = this.lo; i < this.hi; i++) yield i;
  }
}
console.log([...new Range(2, 6).values()]);

class Shadow {
  m() { return "method"; }
}
const s = new Shadow();
s.m = () => "own";
console.log(s.m(), Object.keys(s));

class Box {
  constructor(v) { this.v = v; }
  map(f) { return new Box(f(this.v)); }
}
console.log(new Box(2).map((x) => x * 10).map((x) => x + 1));
const boxes = [new Box(3), new Box(1), new Box(2)];
console.log(boxes.map((b) => b.v));

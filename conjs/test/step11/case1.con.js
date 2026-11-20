// クラス（docs/dev/PLATFORM-PLAN-ja.md の 4.5 節）：コンストラクタ、メソッド、フィールド、static、extends と super
class Point {
  z = 0;
  label = `p${this.z}`;
  constructor(x, y) {
    this.x = x;
    this.y = y;
  }
  norm() { return Math.sqrt(this.x * this.x + this.y * this.y); }
  add(o) { return new Point(this.x + o.x, this.y + o.y); }
  toString() { return `(${this.x}, ${this.y})`; }
  static origin() { return new this(0, 0); }
  static of(x, y) { return new Point(x, y); }
}
const p = new Point(3, 4);
console.log(p.norm(), p.add(new Point(1, 1)).toString());
console.log(p);
console.log(Point.origin(), Point.of(1, 2));
console.log(Point, typeof Point, typeof p);
console.log(Object.keys(p), Object.keys(Point), JSON.stringify(p));
console.log({ ...p });

class Point3 extends Point {
  w = 1;
  constructor(x, y, z3) {
    super(x, y);
    this.z3 = z3;
  }
  norm() { return Math.round(super.norm() * 100) / 100; }
  static origin() { return super.origin(); }
}
const q = new Point3(1, 2, 3);
console.log(q, q.norm(), q.toString());
console.log(Point3.of(5, 6), Point3.origin());
console.log(Point3);
console.log(q instanceof Point3, q instanceof Point, p instanceof Point3, {} instanceof Point);
console.log("norm" in q, "add" in q, "nothing" in q, "origin" in Point3, "of" in Point3);

// アロー関数の this と、コンストラクタを書かない extends
class Counter {
  count = 0;
  makeIncrement() { return () => { this.count++; return this.count; }; }
}
class Named extends Counter {}
const c = new Named();
const inc = c.makeIncrement();
inc(); inc();
console.log(c.count, inc(), c);

// コンストラクタを書かない extends は、引数を親に渡す
class Base {
  constructor(a, b) { this.sum = a + b; }
}
class Derived extends Base {
  describe() { return `sum ${this.sum}`; }
}
console.log(new Derived(2, 5).describe());

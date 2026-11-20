// コンストラクタが返すオブジェクトは new の値になる。エラーの name と toString は、メソッドと同じく記録から見つける
// （this.name = ... は Object.keys に出る）。ランタイムのエラーも、エラーのコンストラクタのメソッドを持つ（M2 のレビュー）
class Ret { constructor() { return { z: 1 }; } }
console.log(new Ret());
class Ret2 { constructor() { this.a = 1; return 5; } }
console.log(new Ret2());
class MyErr extends Error { constructor(m) { super(m); this.name = "MyErr"; } }
const me = new MyErr("bad");
console.log(Object.keys(me), String(me), me.name, me.toString(), JSON.stringify(me));
class E2 extends TypeError {}
const e2 = new E2("x");
console.log(e2.name, String(e2), Object.keys(e2), "name" in e2);
console.log(new Error().toString(), new RangeError("r").toString(), `${new SyntaxError("s")}`);
try { null.x; } catch (e) { console.log(e.toString(), e instanceof TypeError, e instanceof Error, e instanceof RangeError, e.name); }
try { JSON.parse("{"); } catch (e) { console.log(e instanceof SyntaxError, e.name); }

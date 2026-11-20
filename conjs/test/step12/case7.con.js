// 実行時のエラー：関数の new、クラスの new なしの呼び出し、super(...) の前の this、右がクラスでない instanceof
function F() {}
class A {}
class B extends A {
  constructor() {
    this.x = 1;
    super();
  }
}
const cases = [() => new F(), () => A(), () => new B(), () => ({}) instanceof F, () => new (class extends A {})()];
for (const f of cases) {
  try {
    console.log("ok", f());
  } catch (e) {
    console.log(e.name, e.message);
  }
}

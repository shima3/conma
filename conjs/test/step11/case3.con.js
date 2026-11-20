// JavaScript との違い（Node.js と比べない）：取り出したメソッドの this は元のオブジェクトのまま。
// メソッドは読むたびに作るので、o.m === o.m は偽
class Greeter {
  constructor(name) { this.name = name; }
  greet() { return `hello ${this.name}`; }
}
const g = new Greeter("conjs");
const f = g.greet;
console.log(f());
console.log(g.greet === g.greet);

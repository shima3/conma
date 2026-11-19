// then はふつうの名前として使える（ConJS には Promise がないので、then を持つ値はプログラムが作ったものだけ）
const obj = { then: 1, async_name: "ok" };
obj.then = obj.then + 1;
console.log(obj, JSON.stringify({ then: [1] }), obj["then"]);
const thenable = { then: (f) => f(5) };
console.log(thenable.then(v => v * 2));
try { (42).then(); } catch (e) { console.log(String(e)); }

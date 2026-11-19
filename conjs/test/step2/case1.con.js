// 2 つの待つ呼び出しの間は、割り込まれない：共有のオブジェクトを読んで、書くまでの間に、ほかの処理は動かない
const account = { balance: 0 };
function deposit(name, n) {
  for (let i = 0; i < n; i++) {
    const b = account.balance;      // 読む
    let x = 0;
    for (let j = 0; j < 2000; j++) x += j;  // VProc が切り替わるより長く、待たずに回す
    account.balance = b + 1;        // 書く
    host.wait("test.later", 1, name);
  }
}
const a = spawn(() => deposit("a", 5));
const b = spawn(() => deposit("b", 5));
join(a);
join(b);
console.log("balance", account.balance);

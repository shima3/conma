// setTimeout の関数の例外は、join されなければ、プログラムの終わりに報告する（JavaScript のようにすぐ止めない）
setTimeout(() => { throw "nobody joins me"; }, 5);
console.log("main goes on");
sleep(20);
console.log("main ends");

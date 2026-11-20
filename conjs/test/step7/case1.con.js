// sleep：呼んだ処理だけが待ち、その間もほかの処理が動く。Date.now() の差は sleep の長さ以上
// （本当の時計を使うので、負荷で揺れないことだけを確かめる。終わる順は step2 の test.later で確かめる）
let count = 0;
let done = false;
const counter = spawn(() => {
  while (!done) { count++; sleep(5); }
  return "counter stopped";
});
const t0 = Date.now();
sleep(100);
const t1 = Date.now();
console.log("waited at least 100 ms:", t1 - t0 >= 100);
console.log("the other task ran during sleep:", count > 0);
done = true;
console.log(join(counter));
console.log("Date.now() is an integer:", Number.isInteger(t0), typeof t0);
sleep(0);
sleep();
console.log("sleep returns", sleep(1));

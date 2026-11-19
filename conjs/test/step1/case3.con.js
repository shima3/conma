// 待っている間に、ほかの処理が動く。spawn した処理は、呼んだ側が待つまで動かない
let ticks = 0;
const counter = spawn(() => {
  for (let i = 0; i < 3; i++) {
    ticks++;
    console.log("tick", ticks);
    host.wait("test.later", 1);
  }
});
console.log("before wait, ticks =", ticks);
host.wait("test.later", 10, "long");
console.log("after wait, ticks =", ticks);
join(counter);
// spawn した処理の中で、さらに spawn する（FIFO）
const outer = spawn(() => {
  const inner = spawn(() => { console.log("inner"); return 2; });
  console.log("outer");
  return join(inner) + 1;
});
console.log("outer returned", join(outer));

// 端末の道のキー（docs/dev/PLATFORM-PLAN-ja.md の 4.7 節、段階 P6a）：機能 "input" がないとき、nextEvent と pollEvent は
// 標準入力（case1.in）からキーを読む。名前はブラウザの KeyboardEvent.key に合わせる。1 回の読みからいくつものキーを
// 取り出し、列にためる。最後の ESC は、ファイルの終わりで続きが来ないので、時計を待たずに Escape になる。
// 入力が尽きたら、nextEvent は Error を投げる（時間を限っていても、時間を待たない）
let e = nextEvent();                   // 最初のキー。読んだ分は、すべて列に入る
console.log(JSON.stringify(e));
const keys = [e.key];
e = pollEvent();                       // 列にあるので、すぐに返る
console.log(JSON.stringify(e));
keys.push(e.key);
e = nextEvent(1000);                   // 列にあるので、時間を待たずに返る
keys.push(e.key);
try {
  while (true) keys.push(nextEvent().key);
} catch (err) {
  console.log(keys.join(" "));
  console.log(`${err}`);
}
console.log(pollEvent());              // 列は空
try { nextEvent(50); } catch (err) { console.log(`${err}`); }
console.log(host.available("input"));  // 端末の道は、ホストの機能 "input" ではない

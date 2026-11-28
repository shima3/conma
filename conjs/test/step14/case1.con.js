// キーとクリックを待つ（docs/dev/PLATFORM-PLAN-ja.md の 4.6 節、段階 P5c）：nextEvent と pollEvent。wasm 版の VM では、
// case1.host.js が本物の vm/wasm/host/input.js を、手で進める時計で動かす（入力は、呼んだ数で決まり、時計によらない）。
// Rust 版と Python 版の VM には機能 "input" がないので、標準入力（/dev/null）から読む端末の道になり、nextEvent は入力の
// 終わりの Error を投げ、pollEvent は undefined を返す（4.7 節、段階 P6a。段階 P10 から ConJS は wasm 版だけで、ログは case1.log）
console.log(host.available("input"));
if (!host.available("input")) {
  try { nextEvent(); } catch (e) { console.log(`${e}`); }
  try { pollEvent(); } catch (e) { console.log(`${e}`); }
} else {
  try { nextEvent("soon"); } catch (e) { console.log(`${e}`); }
  console.log(nextEvent());                  // 1 回目：ArrowUp。クリックは列に残る
  console.log(pollEvent(), pollEvent());     // 列のクリックと、空の列
  console.log(nextEvent(30));                // 2 回目：時間切れ
  console.log(nextEvent(-5), nextEvent(NaN));  // 3、4 回目：0 ミリ秒
  console.log(nextEvent(10));                // 5 回目：時間切れ。後の x を取らない
  console.log(pollEvent());                  // x は列にある
  const t = spawn(() => nextEvent());        // 7 回目（自分が先に待つ）
  console.log("main", nextEvent());          // 6 回目：先に待ったので a
  console.log("spawned", join(t));           // b
  console.log(nextEvent(Infinity));          // 8 回目
  try { nextEvent(); } catch (e) { console.log(`${e}`); }  // 9 回目：手順がない
}

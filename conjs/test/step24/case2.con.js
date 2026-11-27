// Queue の text()：閉じた後の残りの要素を改行でつなぎ、終わりの改行を除いた 1 つの文字列（Bash の $(...) と同じ）
const q = new Queue();
q.push("a");
q.push(2);
q.push({ k: 1 });
q.close();
console.log(JSON.stringify(q.text()));
// 要素は取られる：もう一度呼ぶと空
console.log(JSON.stringify(q.text()));
// 終わりの改行と空の行は除く（途中のは残る）
const e = new Queue();
e.push("x\n");
e.push("");
e.push("y");
e.push("");
e.push("\n");
e.close();
console.log(JSON.stringify(e.text()));
// 開いている間は、閉じるまで待つ（呼んだ処理だけ）
const w = new Queue();
const t = spawn(() => {
  w.push("one");
  sleep(10);
  w.push("two");
  w.close();
  return "done";
});
console.log(JSON.stringify(w.text()), join(t));
// 親の要素が先に来る
const p = new Queue();
const c = new Queue(p);
c.push("child");
p.push("parent");
p.close();
c.close();
console.log(JSON.stringify(c.text()));
// 空のキュー
const z = new Queue();
z.close();
console.log(JSON.stringify(z.text()));

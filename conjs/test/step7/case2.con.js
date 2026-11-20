// setTimeout(f, ms, ...args)：f を ms ミリ秒の後に、新しい処理で呼ぶ。処理を返すので、join で f の値を受け取れる。
// f が投げた例外は、join でもう一度投げる
const order = [];
const a = setTimeout((x, y) => { order.push("a"); return x + y; }, 30, 2, 3);
const b = setTimeout(() => { order.push("b"); throw "from b"; }, 10);
console.log("typeof the task:", typeof a);
console.log("a:", join(a));
try { join(b); } catch (e) { console.log("b threw:", e); }
console.log("both ran:", order.length);
const c = setTimeout(() => "no delay");
console.log(join(c));

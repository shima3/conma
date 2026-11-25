// ファイルの一番外の async 関数は、ホストの JavaScript として動く（docs/dev/CONJS-HOST-PLAN.md の 34 節）
import { greet, which as whichGreet } from "./lib/greet.con.js";

const limit = 3;

async function later(ms, text) {
  await new Promise((resolve) => setTimeout(resolve, ms));
  console.log(`host: ${text}`);       // プログラムの出力と順にそろう
  return text;
}

async function kinds() {
  return [1, "two", true, { a: [1, 2] }, null, [], -0];
}

async function all(xs) {
  const { twice } = require("./lib/helper.js");
  const ys = await Promise.all(xs.map(async (x) => twice(x)));
  const limit = ys.length;            // 本体の名前は、プログラムの名前を隠す
  return [ys, limit, await which()];
}

async function which() {
  return "case1.con.js";
}

async function fail(kind) {
  if (kind === "reject") throw new Error("rejected on the host");
  return null.x;
}

export async function where() {
  try {
    null.x;
  } catch (e) {
    const at = e.stack.split("\n")[1].split(":");
    return `${at[at.length - 2]}:${at[at.length - 1].replace(")", "")}`;
  }
}

// 呼び出しは待つ。待つ間、ほかの処理が動く
const slow = spawn(() => later(200, "slow"));
console.log(later(10, "fast"));
console.log(join(slow));

console.log(kinds());
console.log(all([1, 2, 3]));
console.log(greet("ConJS"), whichGreet(), which());

// 値として使う。ConJS の関数から呼ぶ
const f = all;
console.log(f([5]));
function onTwo(n) {
  return all([n, n + 1]);
}
console.log(onTwo(10));

// 拒否とホストの例外は、ConJS の Error になる
try { fail("reject"); } catch (e) { console.log(`caught: ${e.message}`); }
try { fail("type"); } catch (e) { console.log(`caught: ${e.message}`); }

// ホストのスタックの行と桁は、このファイルのもの
console.log(where());
console.log(limit);
fail("reject");

// 本体の console.log と、プログラムの console.log の順（run_all.sh は、出力を、すぐには読まないパイプにつないでも流す。
// パイプがいっぱいになると、Node.js の console は後で書く）
async function say(i, text) {
  console.log(`host ${i}: ${text}`);
  console.error(`host error: ${i}`);
  return i;
}
const line = "0123456789".repeat(20);
for (let i = 0; i < 400; i++) {
  console.log(`before ${i}: ${line}`);
  console.log(say(i, line));
}

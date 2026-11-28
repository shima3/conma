// 本体で、プログラムの名前を使うと、変換器の誤り
const scale = 2;
async function f(x) {
  return x * scale;
}
console.log(f(1));

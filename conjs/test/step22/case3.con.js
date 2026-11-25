// 本体で、ConJS の名前（spawn、join、host、sleep など）を使うと、変換器の誤り
async function f() {
  return join(spawn(() => 1));
}

// ConJS のコードの await は要らない
async function f() {
  return 1;
}
console.log(await f());

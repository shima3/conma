// 関数の色がない：配列の map に渡した関数や、ジェネレータの本体の中でも待てる
const doubled = [3, 1, 2].map(n => host.wait("test.later", n, n * 2)[1]);
console.log("map:", doubled);
function* slow() {
  for (const n of [5, 6]) {
    yield host.wait("test.later", 1, n)[1];
  }
}
const seen = [];
const t = spawn(() => { for (const v of slow()) seen.push(v); return seen.length; });
console.log("generator count", join(t), seen);
// try と finally の中で待つ
function guarded() {
  try {
    return host.wait("test.later", 1, "in try")[1];
  } finally {
    console.log("finally:", host.wait("test.later", 1, "in finally")[1]);
  }
}
console.log("guarded:", guarded());

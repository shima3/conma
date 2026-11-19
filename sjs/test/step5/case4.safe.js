// 段階 B9：エラーのオブジェクトで拒否した Promise に受け手がなければ、エラーを作った所の位置（Node.js と同じ）
function make() {
  try { return undefined.y } catch (e) { return e }
}
const err = make()
console.log("made")
Promise.reject(err)

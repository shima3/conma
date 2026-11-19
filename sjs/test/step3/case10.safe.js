// 段階 B9：catch したエラーを後で投げると、投げた所の位置（Node.js と同じ）
let saved
try {
  null.x
} catch (e) {
  saved = e
}
console.log("caught")
throw saved

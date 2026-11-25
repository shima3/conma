// 本体でも、変換器がまだ読めない書き方（正規表現のリテラル）は、まだ受け付けない
async function f(s) {
  return /a+/.test(s);
}

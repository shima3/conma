// 本体でも、字句解析の知らない書き方（正規表現）は、まだ受け付けない
async function f(s) {
  return /a+/.test(s);
}

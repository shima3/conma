// 入力のキューを読み終えずにコマンドが終わっても、後から入れた要素は残る（M1 のレビュー、shima3/conma#96）
const q = new Queue();
$ true <$ q
sleep(10);
q.push("a");
sleep(10);
q.push("b");
q.close();
console.log([...q].join(","));

// 読むのが遅いコマンドに大きな入力を渡しても、Node.js の警告が出ない（パイプの空きを待つたびに、リスナーを外す。M1 のレビュー、shima3/conma#96）
const a = [];
for (let i = 0; i < 30000; i++) a.push(`line ${i} xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`);
$ sh -c "n=0; while read l; do n=$((n+1)); done; echo $n" <$ a

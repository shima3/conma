# エラー：$? を 1 にして続ける
js 'nosuch + 1'
echo "status $?"
js '(' 
echo "status $?"
js '"x"' extra
echo "status $?"
js f= '() => () => 1'
echo "status $?"
js '(() => { const a = ["x"]; a.push(a); return a })()'
echo "status $?"
js '(() => { throw "plain" })()'
echo "status $?"
js sum=
echo "status $?"
js
echo "status $?"
# エラーのときは変数を変えない
v=old
js v= 'nosuch'
echo "v=$v"
# 同じ名前の関数があれば、builtin js で呼ぶ
function js (. rest) { echo "my js" }
js '1'
builtin js '"builtin"'

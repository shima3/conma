# エラー：$? を 1 にして続ける
js 'nosuch + 1'
echo "status $?"
js '(' 
echo "status $?"
js '"x"' extra
echo "status $?"
# 変数に入れるときは、結果の関数（外側の関数を呼んだ () => 1）も JS の値として入り（PLAN-ja.md の 2.5 の 4）、$f で呼べる
js f= '() => () => 1'
echo "status $?"
$f
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

# パイプラインの各要素を別の VProc で動かし、前の要素の標準出力を次の要素の標準入力にする
echo hello | cat
printf 'b\na\nc\n' | sort
function f () { echo one; echo two; }
f | cat
# 関数の中のパイプラインの出力も、外のパイプに入る
function g () { echo a | cat; echo b; }
g | cat
if true; then echo in-if; fi | cat
for x in 1 2; do echo "for $x"; done | cat
echo x | cat | cat
# $? は最後の要素の終了ステータス
false | true; echo "status $?"
true | false; echo "status $?"

# トップレベルの外部コマンドには、シェルの標準出力をそのまま渡す（パイプにならない。端末なら端末のまま）
sh -c '[ -p /dev/fd/1 ] && echo pipe || echo not-pipe'
# 改行で終わらない出力も、そのまま出る
printf 'no-newline'
echo
printf 'a\r\n'
echo after
# 関数の出力が端末のままなら、関数の中の外部コマンドも直接書く（段階 10。改行で終わらない出力もそのまま）
function f () { sh -c '[ -p /dev/fd/1 ] && echo pipe || echo not-pipe'; printf 'x'; }
f
echo
# 外部コマンドに渡すパイプは OS のパイプなので、そのまま書く（段階 15）
f | cat
# $( ) と変数リストへのリダイレクトも、今までどおり
x=$(printf 'y')
echo "[$x]"
sh -c 'echo out' > (o)
echo "o=$o"

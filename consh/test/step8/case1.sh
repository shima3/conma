# トップレベルの外部コマンドには、シェルの標準出力をそのまま渡す（パイプにならない。端末なら端末のまま）
sh -c '[ -p /dev/fd/1 ] && echo pipe || echo not-pipe'
# 改行で終わらない出力も、そのまま出る
printf 'no-newline'
echo
printf 'a\r\n'
echo after
# 関数の中の外部コマンドの出力は、今までどおり行ごとの要素になる
function f () { sh -c '[ -p /dev/fd/1 ] && echo pipe || echo not-pipe'; printf 'x'; }
f
# $( ) と変数リストへのリダイレクトも、今までどおり
x=$(printf 'y')
echo "[$x]"
sh -c 'echo out' > (o)
echo "o=$o"

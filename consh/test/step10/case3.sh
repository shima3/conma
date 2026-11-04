# リダイレクト付きの if 文と for 文も、中の外部コマンドはファイルに直接書く
d=$1
if true; then printf 'if-raw'; fi > $d/out
for i in 1 2; do printf "$i"; done > $d/out2
cat $d/out $d/out2
echo
# $( )、変数リスト、パイプでは、今までどおり行ごとの要素になる
function f () { printf 'a\r\nb'; }
x=$(f)
echo "[$x]"
f > (p q)
echo "[$p][$q]"
f | od -c | sed 's/  */ /g'

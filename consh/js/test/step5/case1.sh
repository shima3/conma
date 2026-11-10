# js 変数名= は、結果を JS の値のまま変数に入れる。$ で読むたびに、その時の値を文字列かリストにする
js o= 'globalThis.__o = {a: 1}'
echo "o=$o"
js 'void setTimeout(() => { __o.a = 2 }, 0)'
sleep 0.1
echo "o=$o"
js wv= '[1, 2]'
echo "wv=$wv"
for x in $wv; do echo "[$x]"; done
# 数や真偽値は、表に入れずに値のまま持つ
js k3= '0.1 + 0.2'
js t3= 'true'
echo "$k3 $t3"
# 関数は $f で呼ぶ（js で呼び、結果を標準出力に出す）。js は値が関数なら呼ぶので、関数を返す関数を書く
js add= '() => (a, b) => Number(a) + Number(b)'
$add 1 2
# シェルで代入し直すと String になる
o=plain
echo "o=$o"
# 先頭の語で JavaScript とする文は、もうない（シェルのコマンドとして探す）
n = 1
echo "status $?"

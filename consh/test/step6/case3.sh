# 2> (b)：標準エラー出力の値。標準出力と別の変数リストなら、並行に読む
sh -c 'echo out; echo err >&2' > (o) 2> (e)
echo "o=$o e=$e"
function e () { echo out; echo err 1>&2; }
# 2> (b) 1>&2：標準出力か標準エラー出力の早い方の1つ目
e 2> (b) 1>&2
echo "b=$b"
# 1>&2 2> (c)：標準出力は元の標準エラー出力に出て、c は標準エラー出力の1つ目
e 1>&2 2> (c)
echo "c=$c"

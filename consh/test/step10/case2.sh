# 関数の出力をファイルに向けると、中の外部コマンドはバイト列をそのままファイルに書く
d=$1
function f () { echo begin; printf 'a\r\nno newline'; echo; echo end; }
f > $d/out
od -c $d/out | sed 's/  */ /g'
# 要素の LazyList は、リダイレクトを書いた文の OFS で文字列化する
l=(x y)
function g () { echo ${l}; }
OFS=+ g > $d/out2
cat $d/out2
# 2>&1 も同じファイルに直接書く
function e () { sh -c 'echo err >&2; echo out'; }
e > $d/out3 2>&1
cat $d/out3

# glob は外部コマンド、関数、組み込みコマンドの引数と for 文の語に適用する。一致しなければ語を残す
ls gdir/*.txt gdir/*.none
function f () { echo; }
f gdir/*.txt
echo gdir/?.se "gdir/*.txt" gdir/\*.txt gdir/.*.txt
for x in gdir/*/*.txt; do echo $x; done
# 変数定義の右辺には適用しない。値 (語 ...) には適用する
s=gdir/*.txt
a=(gdir/*.se)
echo $s ${a}
# ${-語} は一致した名前の LazyList。一致しなければ空。記号がなければ語のまま
f ${-gdir/*.txt} ${-gdir/*.none} ${-gdir/c.se} "${-gdir/*.se}"
# 外部コマンドには葉ごとの語にする。その葉には、もう一度 glob を適用しない
ls ${-gdir/x*}

# 中断や kill で文の残りを捨てても、リダイレクトで開いたファイルを閉じる（lsof でシェルが開いているファイルを数える）
d=$1
function opened (name) { n=$(lsof -p $$ 2> /dev/null | grep -c "$name"); echo "$name open: $n"; }
# パイプの読み手が先に終わり、書き手の for 文を中断する
for i in $(yes); do echo $i; done 2> $d/abort-err | head -1
opened abort-err
# 変数リストが一杯になり、コマンドを止める
for i in $(yes); do echo $i; done 2> $d/capture-err > (a)
echo "a=$a"
opened capture-err
# kill でジョブを止める
for i in $(yes); do echo $i; done > $d/killed &
j=$!
kill $j
wait $j
echo "killed $?"
opened killed
# exit で、パイプラインの要素の VProc を終える
echo x | for i in a; do exit 3; done 2> $d/exit-err
echo "exit $?"
opened exit-err
# 普通に終わる文は、今までどおり閉じる
echo done > $d/normal
opened normal

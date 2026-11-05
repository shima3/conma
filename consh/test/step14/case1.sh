# レビュー 1：LazyList を読んでいる途中のジョブを kill しても、その LazyList は読める（評価を終えてから止まる）
# $( ) の出力は 2 秒後に終わるので、速い VM でも、1 秒後の kill のときジョブはまだ $x を評価している。
# 要素ごとに外部コマンドを起動して、読むのを書くより遅くする（読むのが速いと、書き手が場所を待たないので、
# 文の終わりに $( ) が終わるまで待ち、kill の前にジョブが終わる）
x=$(seq 1 2000; sleep 2)
function eat (l) { for i in $l; do true $i; done; }
eat $x &
j=$!
sleep 1
kill $j
wait $j
echo "killed $?"
echo $x | wc -w | tr -d ' '

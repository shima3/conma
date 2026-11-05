# レビュー 1：LazyList を読んでいる途中のジョブを kill しても、その LazyList は読める（評価を終えてから止まる）
# $( ) の出力は 2 秒後に終わるので、速い VM でも、1 秒後の kill のときジョブはまだ $x を評価している
x=$(seq 1 2000; sleep 2)
function eat (l) { for i in $l; do echo $i > /dev/null; done; }
eat $x &
j=$!
sleep 1
kill $j
wait $j
echo "killed $?"
echo $x | wc -w | tr -d ' '

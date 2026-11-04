# レビュー 9：パイプラインのジョブの wait は、すべての要素を待つ
d=$1
function slow () { sleep 1; echo first-done >> $d/f; }
slow | /usr/bin/true &
wait
cat $d/f
# レビュー 10：終わったジョブを kill しても、wait は実際の終了ステータスを返す
sh -c 'exit 3' &
j=$!
sleep 1
kill $j
wait $j
echo "finished job $?"

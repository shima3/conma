# kill で止めたジョブを wait すると 143
sleep 30 &
k=$!
kill $k
wait $k
echo "killed $?"
# 関数のジョブも止められる
function forever () { for x in $(yes); do echo $x > /dev/null; done; }
forever &
f=$!
kill $f
wait $f
echo "killed function $?"
# ないジョブ
kill 9999
echo "no job $?"

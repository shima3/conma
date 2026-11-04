# kill で止めたジョブの、パイプラインの前の要素と $( ) の VProc も止める。止まれば、ログはもう伸びない
d=$1
function grows (name) { a=$(wc -c < $d/$name); sleep 1; b=$(wc -c < $d/$name); if test "$a" = "$b"; then echo "$name: stopped"; else echo "$name: still growing"; fi; }
# パイプラインのジョブ：最後の要素は外部コマンド、前の要素は ConSh の関数。前の要素はパイプに書かないので、
# 読み手がいなくなっても止まらない（kill がパイプラインのすべての要素を止める）
function quiet (log) { for i in $(yes); do echo $i >> $d/$log; done; }
quiet pipe-log | cat > /dev/null &
j=$!
sleep 1
kill $j
wait $j
echo "pipe job $?"
grows pipe-log
# $( ) を待っているジョブ：$( ) の中のループが止まる
function waiter () { x=$(quiet subst-log); echo never; }
waiter &
j=$!
sleep 1
kill $j
wait $j
echo "subst job $?"
grows subst-log

# レビュー 2：文の終わりにファイルを閉じても、& のジョブは自分の複製に書き続ける（Bash と同じ）
d=$1
function g () { sleep 1; echo late; /bin/echo ext-late; }
function f () { echo early; g & }
f > $d/out
wait
cat $d/out

# exit はスクリプトでも使える。パイプラインの要素と $( ) の中では、その VProc だけを終える
x=$(echo in; exit 3; echo never)
echo "x=$x $?"
echo a | exit 2
echo "pipe $?"
function f () { echo f; exit 4; echo never; }
f
echo never

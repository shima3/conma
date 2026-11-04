#!/bin/bash
cd ${0%/*}
. ../common.sh
d=$(mktemp -d /tmp/consh.step13.XXXXXX)
run case1 case1.sh $d < /dev/null
rm -rf $d
# case2：対話モードで、エラーと Ctrl-C（& の sh がシェルに SIGINT を送る）でコマンドの残りを捨てたときも閉じる
run case2 < case2.input

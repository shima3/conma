#!/bin/bash
cd ${0%/*}
. ../common.sh
require lsof
# case1、case3：中断、exit、kill で文の残りを捨てても、開いたファイルを閉じ、起動した VProc を止める
d=$(mktemp -d /tmp/consh.step13.XXXXXX)
run case1 case1.sh $d < /dev/null
run case3 case3.sh $d < /dev/null
rm -rf $d
# case2、case4：対話モードで、エラーと Ctrl-C（& の sh がシェルに SIGINT を送る）でコマンドの残りを捨てたときも同じ
run case2 < case2.input
run case4 < case4.input

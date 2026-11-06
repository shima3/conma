#!/bin/bash
cd ${0%/*}
. ../common.sh
# 段階 20：関数値、bind、reset と shift
# case1：関数値。case2：bind。case3：関数値を文字列にするエラーなど（対話モードで、エラーの後も続ける）
run case1 case1.sh < /dev/null
run case2 case2.sh < /dev/null
run case3 < case3.input
# case4：reset と shift。case5：reset と shift のエラー（対話モード。前のコマンドで取った継続は、別の VProc なので呼べない）
run case4 case4.sh < /dev/null
run case5 < case5.input

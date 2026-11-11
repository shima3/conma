#!/bin/bash
# wasm 版の VM を macOS の JavaScriptCore のシェル（jsc、vm-jsc.js）で動かす。ほかのテストは Node.js で動かす
# （consh/js/PLAN-ja.md の 2.4）ので、jsc で動くことはここだけで確かめる。jsc がなければ（macOS 以外）、何もしない。
# ConSh は始めに JavaScript が使えるかを __JS_call__ で調べ、その結果は Promise で VM に渡るので、
# vm-jsc.js が Promise を動かさないと、VProc が待ったまま止まる（every VProc waits）。
# 時間を待つ JavaScript（setTimeout）は、jsc のイベントループに戻らないと動かない
cd ${0%/*}
jsc="${JSC:-/System/Library/Frameworks/JavaScriptCore.framework/Versions/Current/Helpers/jsc}"
[ -x "$jsc" ] || exit 0
export CONMA_VM=wasm
export CONSH_IMPL=conma
export CONMA_WASM_HOST=jsc
unset CONSH_ENABLE_JS
. ../../../test/common.sh
# スクリプト
run case1 case1.sh < /dev/null
# 対話モード
run case2 < case2.in
# jsc はモジュールを作れない（__VProc_spawn__）ので、(( )) と $(( )) はホストの JavaScript で評価する
(export CONSH_ENABLE_JS=sjs; run case3 case3.sh < /dev/null)
# 組み込みコマンド js
(export CONSH_ENABLE_JS=js; run case4 case4.sh < /dev/null)

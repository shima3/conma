#!/bin/bash
cd ${0%/*}
. ../common.sh
# 段階 21：SJS の (( )) と $(( )) を、ConMa に変換して動かす（consh/SAFEJS-PLAN-ja.md の段階 B4。consh/sjs/README-ja.md）。
# JavaScript のない版（ネイティブの VM）でも動く。Rust 版の ConSh には (( )) がないので、確かめない
[ "$CONSH_IMPL" = rust ] && exit 0
# case1：値、SJS変数とその写し、$名前、関数、メソッド、console.log とリダイレクト、SJS関数をコマンドとして呼ぶ
run case1 case1.sh "$tmp" < /dev/null
# case2：エラー（$? は 2）と、範囲の外のコード
run case2 case2.sh < /dev/null
# case3：語の中の $(( )) のエラーは、スクリプトを終了ステータス 2 で終える
run case3 case3.sh < /dev/null
# case4：ジェネレータ（段階 B5）。別の (( )) で続け、シェルの値としては、読むときに next を呼ぶ LazyList になる
run case4 case4.sh < /dev/null
# case5：async と Promise（段階 B5b）。評価ごとにマイクロタスクの列を最後まで動かし、シェルの値の Promise は、その結果になる
run case5 case5.sh < /dev/null

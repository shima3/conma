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
# case6：async ジェネレータと for await、タグ付きテンプレート、配列の穴と length、let の重なり
run case6 case6.sh < /dev/null
# case7：対話モードでは、コマンド行の文がそのまま (( )) なら、値を標準エラーに出す（ループ、関数、if の条件の中は出さない）
run case7 < case7.input
# case8：エラーの位置（段階 B9）。(( )) の中の失敗した所を、スクリプトの行と桁で出す
run case8 case8.sh < /dev/null
# case9：数の関数の wasm 版（consh/SJS-WASM-PLAN-ja.md の段階 4）。wasm 版の VM では WebAssembly で、ほかの VM では ConMa 版で動き、
# 同じログになる。後の (( )) が名前や Math のメンバーを書き換えると、ConMa 版で動く
run case9 case9.sh < /dev/null
# case10：配列を使う数の関数（consh/SJS-WASM-ARRAY-PLAN-ja.md の段階 A4）。wasm 版の VM とほかの VM で同じログ
run case10 case10.sh < /dev/null
# case11：呼ぶときに確かめる Math のメンバーは、使うものだけ（呼ぶ数の関数が使うものも）。wasm 版の VM とほかの VM で同じログ
run case11 case11.sh < /dev/null

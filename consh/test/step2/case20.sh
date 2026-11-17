# 引用符の語（要素がみな引用符の中か、glob の記号のないリテラル）：外部コマンドの語、関数と組み込みコマンドの引数、
# 変数定義の右辺として、つないだ String 1 つになる（最適化版は変換のときに組む。Quoted_word_code）
a=1; b='x y'; e=''; l=$(seq 1 3); g='*'
function show (x y) { echo "[$x][$y]"; }
function f () { echo hi; }
echo "x$a" "$b" "" "$e" 'q'"$a" a"$b"c \$"$a" "${a}z" "$l" "pre$(echo sub)post" "${u-def}" "$g"
show "" "$e"
show "$b" x"$e"
printf '<%s>\n' "x$a" "$e" a"$b"c "$l" "$g" "${a}"
m="x$a"; n="$l"; o="$e"; p=a"$b"c; q="${u-df}"; r='s'"$a"
echo "$m|$n|$o|$p|$q|$r"
for w in "$l" x"$a" ""; do echo "w=[$w]"; done
k=$(echo "in $a")
echo "$k" "f: $(f)"
# glob の記号のあるリテラルを含む語は、今までどおり glob を試す（一致しなければそのまま）
echo "$a"zz*nomatch
# 関数値は文字列にできない（位置は "..." の中の $h）
h=function () { echo hi; }
echo "a$h"

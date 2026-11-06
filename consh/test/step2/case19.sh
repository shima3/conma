# 部分が2つ以上の語を、関数の引数として展開する（部分がみな String なら、連結した1つの語）。
# 空の語は、引用符か ${x} があるときだけ残る
function show (. args) { for a in $args; do printf '[%s]' $a; done; echo " n=" ; }
e= a=ab b=x m='*'
show $a$b $e$e "$e"$e ${e}$e ${e}${e} $e"" x$e $a$e ${a}-${b} "$a"$b'q' $a"$m" "$a"zz$m
l=$(seq 1 2)
show $a$l $l$b ${l}$a

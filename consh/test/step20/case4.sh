# reset と shift：抜けるだけ、値を渡して何度も続ける、ジェネレータ、関数の中の shift、for 文の途中の継続、
# 変数を巻き戻さないこと、入れ子、続けた中の shift、B の中の return、$?、パイプと &
function find (want l) { for x in ${l}; do if test "$x" = "$want"; then shift k { echo "found $x"; }; fi; done; echo "not found"; }
l=(a b c)
reset { find b ${l} }
reset { find z ${l} }
reset { echo start; shift k (v) { $k 1; $k 2; }; echo "v=$v"; }
echo "status $?"
function gen () { for i in a b c; do shift k { echo $i; echo $k; }; done; shift k { echo end; echo $k; }; }
function consume (v next) { if test "$v" = end; then return 0; fi; echo "got $v"; $next > (v2 next2); consume $v2 $next2; }
reset { gen } > (v next)
consume $v $next
function gen () { for i in a b c; do shift k { echo $i; echo $k; }; done; echo end; echo none; }
function consume (v next) { if test "$v" = end; then return 0; fi; echo "got $v"; $next > (v2 next2); consume $v2 $next2; }
reset { gen } > (v next)
consume $v $next
# for 文の途中で取った継続を、後で何度も呼ぶ
reset { for i in $(seq 1 3); do shift k { echo $k; }; echo "i=$i"; done; echo loop-end; } > (k1)
$k1 > (a k2)
echo "a=$a"
$k1 > (b k3)
echo "b=$b"
$k2 > (c k4)
echo "c=$c"
$k4
# 変数は巻き戻さない
reset { n=0; shift k { echo $k; }; n=$(expr $n + 1); echo "n=$n"; } > (kn)
$kn
$kn
$kn
# reset の入れ子と、続けた中の shift
reset { echo outer-start; reset { shift k { echo inner-B; $k; echo inner-after; }; echo inner-rest; }; echo outer-rest; }
reset { shift k (a) { $k x > (r1); $k y > (r2); echo "r1=$r1 r2=$r2"; }; shift j { echo "seen $a"; }; echo never; }
# B の中の return
reset { shift k { return 4; echo never; }; }
echo "status $?"
# 何もしない reset と、$?
reset { false; }
echo "status $?"
reset { }
echo "empty $?"
reset { echo piped; shift k { echo B-piped; $k; }; echo rest-piped; } | cat
reset { shift k { echo bg; }; } &
wait $!
echo "job $?"

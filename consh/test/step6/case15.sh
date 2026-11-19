# rest のある受け取りの LazyList（最適化版は、読まれたときに MSeq から 1 回で作るサンク 1 つ。LazyList_from_mseq）と、
# for 文などで $rest を語にする所（最適化版は、バイト数を数えないとき、評価済みの要素を 1 回でたどる。Ready_fields。
# 先頭がサンクなら評価してから。Rest_fields）
seq 1 5 > (a . rest)
n=0; for x in $rest; do n=$x; done; echo "a=$a n=$n"
for x in $rest; do echo "x=$x"; done
echo one > (a . rest)
echo "a=$a rest=[$rest]"
for x in $rest; do echo "never $x"; done
# 名前の数より少ない要素
seq 1 1 > (a b . rest)
echo "a=$a b=[$b] rest=[$rest]"
# 記録の要素の後に、stdin が渡した残り（"direct"）が続く（wasm 版の VM では、外部コマンドの後の stdin が
# 何も読まないので、要素は echo で書く）
function g () { echo 1; echo 2; stdin; }
function h () { g > (a . rest); for x in $a $rest; do echo "d=$x"; done; }
printf 'p\nq\n' | h
# glob の記号を含む要素（一致すればパス、しなければそのまま）
printf '/dev/nul?\nno*such*file\nplain\n' > (a . rest)
for x in $a $rest; do echo "g=$x"; done
# ${rest} は 1 つの値（glob しない）
for x in ${rest}; do echo "b=$x"; done
# $( ) の値（評価していない要素から先は、前と同じ道）。先頭を読んでから
v=$(seq 1 4)
echo $v > (first . others)
for x in $v; do echo "v=$x"; done
# 外部コマンドの引数（バイト数を数える）
seq 1 3 > (a . rest)
printf '[%s]' $rest; echo
# 読まずに置き換えた rest と、2 回目に読む rest（評価済み）。関数の引数と "$rest" と、ほかの変数に写した rest
seq 1 3 > (a . rest)
rest=gone
echo "rest=$rest"
seq 4 6 > (a . rest)
copy=${rest}
for x in $copy; do echo "c=$x"; done
for x in $rest; do echo "r=$x"; done
function f (x y) { echo "f x=$x y=$y"; }
f $rest
echo "q=$rest"

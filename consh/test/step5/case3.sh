# パイプラインの要素の中の変数定義、export、cd は外に影響しない。$? は外の値から始まる
here=$(pwd)
if true; then v=1; export E=2; cd /; fi | cat
echo ${v-unset} ${E-unset}
if [ "$(pwd)" = "$here" ]; then echo same-dir; fi
false; true | echo "in pipe $?"
# LazyList の要素は、パイプを書いた文のスコープの OFS で文字列化する
OFS=,
l=(a b)
echo ${l} | cat

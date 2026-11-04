# & の中の変数定義、cd は外に影響しない
here=$(pwd)
if true; then v=1; cd /; fi &
wait
echo ${v-unset}
if [ "$(pwd)" = "$here" ]; then echo same-dir; fi

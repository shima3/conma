# 無限の LazyList を外部コマンドの引数に展開しても、上限を超えたところでエラーになる
x=$(yes)
export ARG_MAX=1000
echo before
printf %s $x
echo not reached

# ARG_MAX は環境変数だけを見る。同じ名前の局所変数は無視する
ARG_MAX=5
printf '%s\n' local-ignored
# export すると子プロセスにも渡る
export ARG_MAX=100
sh -c 'echo "child: $ARG_MAX"'
# 関数と組み込みコマンドの引数には適用しない
function f (a) { echo "f: $a"; }
ARG_MAX=5 f long-argument
ARG_MAX=5 echo builtin-echo
# command で起動した外部コマンドには適用する
ARG_MAX=5 command printf x
echo not reached

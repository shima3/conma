# 関数、組み込みコマンド、外部コマンドの区別と、前置の変数定義の反映
function f (. rest) { echo $rest }
l=(a "" b) s=""
f ${l} $l x${l}y ${s} $s "$s"
cmd ${l} $l x${l}y ${s} $s "$s"
echo ${l}
command printf ${l}
builtin echo ${l}
OFS=, f "$l" ${OFS-none}
OFS=, cmd "$l"
x=5
x=1 cmd $x
y=1 cmd $y
a=1 b=${a-none} cmd $b

# 関数と組み込みコマンドへの値の受け渡し、前置の変数定義の反映、OFS、defined、先頭の語の LazyList、リダイレクト付きの if 文
d=$1
function show (a . rest) { echo "a=[$a] rest=[$rest]"; echo ${a} > $d/out1; }
l=(x "" y) e=() s=""
show ${l} $l
cat $d/out1
show ${e} $e z
show ${s} $s w
show x${l}y
echo ${l}
echo a ${l}
OFS=: echo "$l"
OFS=- echo ${l}
OFS=+
echo ${l}
x=5
x=1 sh -c 'echo child x=$x'
V=v sh -c 'echo child V=$V'
V=v echo "prefix ${V}"
a=1 b=${a-none} echo "b=$b"
defined x; echo "x $?"
defined nope; echo "nope $?"
HOME=/tmp defined -l HOME; echo "-l HOME $?"
HOME=/tmp defined HOME; echo "HOME $?"
cmd=(echo hello world)
${cmd}
$cmd
command printf "%s|" ${l}; echo
builtin echo ${l}
function f () { echo inner; sh -c 'echo ext' }
if true; then f; sh -c 'echo ext2'; fi > $d/out2
cat $d/out2

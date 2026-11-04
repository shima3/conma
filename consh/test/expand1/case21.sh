# 先頭の語が LazyList の場合
function f (. rest) { echo $rest }
l=(f b "" c) m=(cmd b c) s=""
${l} z
$l z
${m} z
$s cmd z
n=$(echo b; echo ${l})
k=$(echo ${l}; echo ${m}; echo z)
${k} y
x${l} z
e=()
${e} z

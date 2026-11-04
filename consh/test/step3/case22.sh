# 関数の中の ${1-def} は既定値を使わずにエラーになる
function f () { echo ${1-def}; }
f a

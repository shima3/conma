# 関数の中の ${@-x} は既定値を使わずにエラーになる
function f () { echo ${@-x}; }
f

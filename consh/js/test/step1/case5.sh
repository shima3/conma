# CONSH_ENABLE_JS がないとき：js は、引数を確かめた後にエラーにし、$? を 2 にする。JS の文は使わず、シェルの文になる
js
echo "status $?"
js '1 + 2'
echo "status $?"
js sum= '(a, b) => a + b' 3 4
echo "status $?"
defined sum
echo "defined sum: $?"
n = 1 + 2
echo "status $?"

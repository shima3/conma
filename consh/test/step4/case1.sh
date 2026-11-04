# コマンド置換の値、OFS、関数の出力、先頭の語、外に影響しないこと、\r\n、標準エラー出力
l=$(printf 'a\nb\n\nc')
for x in ${l}; do echo "[$x]"; done
echo "$(echo a; echo b)"
OFS=$'\n'
echo "$(echo a; echo b)"
OFS=" "
function f () { echo one; echo ${l}; sh -c 'echo ext' }
m=$(f)
for x in ${m}; do echo "<$x>"; done
$(echo echo hi) there
c=(echo hi)
$(echo ${c}) there
x=1
d=$(pwd)
y=$(x=2; export E=1; cd /; function g () { echo g }; echo $x; pwd)
echo "x=$x y=$y E=${E-unset}"
if test "$(pwd)" = "$d"; then echo "pwd kept"; fi
defined g; echo "g $?"
false
echo "$(true) status $?"
echo nested $(echo $(echo inner))
w=$(printf 'r1\r\nr2\r\n')
for x in ${w}; do echo "{$x}"; done
e=$(sh -c 'echo to-stderr 1>&2')
echo "e has ${e-none}"

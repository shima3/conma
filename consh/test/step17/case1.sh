# JavaScript のない版の js：引数を確かめた後、メッセージを出して $? を 127 にする
js
echo "status $?"
js sum=
echo "status $?"
js '1 + 2'
echo "status $?"
x=3
js sum= '(a, b) => a + b' $x 4
echo "status $?"
defined sum
echo "defined sum: $?"
inner=(b c)
list=(a ${inner})
js '(l) => l' ${list} $list
echo "status $?"
# 「識別子 =」でなければコード
js 'x=1'
echo "status $?"
js 1x= 2
echo "status $?"

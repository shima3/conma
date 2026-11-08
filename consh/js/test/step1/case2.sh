# 変数に入れる：js 変数名= コード {引数}
x=5
y=7
js sum= '(a, b) => Number(a) + Number(b)' $x $y
echo "sum=$sum"
js list= '[1, [2, 3], []]'
for e in $list; do echo "list: [$e]"; done
js empty= '[]'
for e in $empty; do echo "never"; done
echo "empty done"
js u= 'undefined'
echo "u=[$u]"
js s= '"one\ntwo"'
echo "s=[$s]"
# 引数：String と LazyList（配列）。$list は葉ごと、${list} は1つの配列
js '(...a) => a.length + ": " + JSON.stringify(a)' ${list}
js '(...a) => a.length + ": " + JSON.stringify(a)' $list
js '(s) => s.toUpperCase()' 'héllo 日本'
# 関数の中の変数は外に出ない
function f () { js inner= '"in f"'; echo "inside: $inner" }
f
defined inner
echo "defined inner: $?"

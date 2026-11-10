# js の JS の値を、SJS の $名前 で値のまま読む（consh/SAFEJS-PLAN-ja.md の 2.2 の 7）
js o= '({a: 1, b: [1, 2, 3]})'
echo "$(( $o.a + 1 )) $(( $o.b.length ))"
(( keep = $o ))
js 'void 0'
echo "$(( keep.a ))"
# SJS の値を変数に入れれば、$ で読める
v=$(( { s: `x${2}` } ))
echo "v=$v"

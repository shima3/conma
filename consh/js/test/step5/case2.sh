# JS の値の表：どの変数も持たなくなった値は、表から消える（数は表に入れない。__consh_value.count() は表の大きさ）
js n= '1'
js '__consh_value.count()'
js o= '({a: 1})'
js '__consh_value.count()'
o=plain
echo "o=$o"
js '__consh_value.count()'
# 関数の局所変数の値も、関数から戻れば消える（2026-11-17 から。前は、覚えておく文のコードが初めて実行したときの
# 継続を持ち、関数の文脈が循環して、VM の循環の回収が走るまで残った。docs/consh/dev/COMPILE-PLAN-ja.md の 5）
function make () {
  js t= '({b: 2})'
  echo "in make: $t"
}
make
echo "made"
js '__consh_value.count()'
js f= '() => () => 5'
$f
f=gone
echo "f=$f"
js '__consh_value.count()'

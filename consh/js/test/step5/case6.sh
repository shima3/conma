# JS の値の表：どの変数も持たなくなった値は、表から消える（数は表に入れない。__consh_value.count() は表の大きさ）
n = 1
__consh_value.count()
o = {a: 1}
__consh_value.count()
o=plain
echo "o=$o"
__consh_value.count()
# 関数の局所変数の値は、関数の文脈が循環して残るので、VM の循環の回収が走るまで残る（ここでは 1 のまま）
function make () {
  t = {b: 2}
  echo "in make: $t"
}
make
echo "made"
__consh_value.count()
js f= '() => () => 5'
$f
f=gone
echo "f=$f"
__consh_value.count()

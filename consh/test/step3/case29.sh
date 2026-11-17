# break も continue もない for 文（最適化版は、ループの文脈と、繰り返しごとの継続を作らない）
# break のあるループの中の、break のないループ
for x in a b c; do
  for y in 1 2; do echo "$x$y"; done
  if test $x = b; then break; fi
done
# break のないループの中の、break のあるループ。外の値の $( ) を読みながら
for x in $(printf "p\nq\n"); do
  for y in 1 2 3; do
    if test $y = 2; then continue x; fi
    echo "$x$y"
  done
  echo "not here $x"
done
# 関数の中の、break のないループ。関数の外のループの中から呼ぶ
function f (n) { r=""; for i in $(seq 1 $n); do r="$r$i"; done; echo $r; }
for x in 2 3; do f $x; done
# break のないループの中で定義した関数の break は、関数の外のループを指さない（エラー）
for x in a; do
  function g () { break; }
  echo "before g"
  g
  echo "after g"
done
echo "end"

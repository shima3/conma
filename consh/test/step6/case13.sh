# 変数リストへの受け取りだけの、先頭の語が決まっている単純文（最適化版は、変換のときに組み込みコマンドか外部コマンドかを
# 決める。Cmd_capture_code）：同じ文を、関数を定義する前と後で実行すると、後では関数を呼ぶ
function twice () { for k in 1 2; do
  echo "e$k" > (a); printf '%s\n' "p$k" > (b); echo "a=$a b=$b"
  if [ $k = 1 ]; then
    function echo (. x) { builtin echo "my echo: $x"; }
    function printf (f . x) { builtin echo "my printf: $x"; }
  fi
done; }
twice
# 外部コマンド、組み込みコマンド、標準エラー出力、変数の数より多い出力、rest
seq 1 3 > (one . more)
echo "one=$one more=$more"
seq 1 5 > (x y)
echo "x=$x y=$y status=$?"
function warn (m) { echo "out $m"; echo "err $m" 1>&2; }
warn hi 2> (e) > (o)
echo "o=$o e=$e"
# 外部コマンドの標準エラー出力
sh -c 'echo to-err 1>&2; exit 3' 2> (le)
echo "status=$? le=$le"
# 関数の引数の数の誤り（受け取りの中でもエラー）
function two (p q) { echo "$p $q"; }
two 1 2 > (r)
echo "r=$r"
two 1 > (r)

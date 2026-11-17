# 変数リストへのリダイレクトだけの文（最適化版は、変換のときに分けておく）
function both () { echo out1; echo err1 1>&2; echo out2; }
both > (o) 2> (e)
echo "o=$o e=$e"
both 2> (e) > (o p)
echo "o=$o p=$p e=$e"
# 変数の数より多い出力：変数の数だけ受け取って、コマンドを止める（$? は 0）
function many () { echo 1; echo 2; echo 3; echo never; }
many > (a b)
echo "a=$a b=$b status=$?"
# rest のある変数リストと並べる
both > (first . rest) 2> (e)
echo "first=$first e=$e"
for x in ${rest}; do echo "rest $x"; done
# 同じ出力に 2 つ向けるのはエラー
echo a > (x) > (y)
echo "not here"

# 外部コマンドの出力を、行ごとにターゲットに書く（最適化版は、LazyList を作らずに読み、rest のある変数リストの
# 受け取りには、記録に直接加える。Target_pump、Pump_lines、Target_line_writer）
# 空の行、\r\n、最後に改行のない行
printf 'a\n\nb\r\nc' > (x . rest)
echo "x=[$x] rest=[$rest]"
printf '' > (x . rest)
echo "x=[$x] rest=[$rest]"
printf 'only' > (x y . rest)
echo "x=[$x] y=[$y] rest=[$rest]"
# 1024 文字の断片をまたぐ行（2000 文字の行の後に 1 行）
awk 'BEGIN { s = ""; for (i = 0; i < 2000; i++) s = s "z"; print s; print "tail" }' > (long . rest)
echo "$long" | awk '{ print "long=" length($0) }'; echo "rest=$rest"
# rest のない変数リスト（一杯になれば止める）と、2 つの記録の組
seq 1 5 > (a b)
echo "a=$a b=$b"
seq 1 5 > (a b . rest) 2> (e . er)
echo "a=$a b=$b rest=$rest e=[$e]"
# 関数の中の外部コマンドと echo が、同じ受け取りに書く
function f () { seq 1 3; echo four; seq 5 6; }
f > (a . rest)
echo "a=$a rest=$rest"
# & の受け取り（遅延の値と、伸びる rest）
seq 1 4 > (p . r) &
wait
echo "p=$p r=$r"
# パイプとファイル
seq 1 3 | for i in $(stdin); do echo "i=$i"; done
seq 1 2 > /dev/stdout

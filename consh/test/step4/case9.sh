# 文の終わりの待ち：for 文の値の $( ) が終わらないうち（読み手が読むまで書き手が待つ）に、本体で起動した $( ) は、
# その文の終わりに終わるまで待つ（標準エラー出力の行が、次の文の echo より先に出る）
for i in $(seq 1 100000); do
  y=$(sh -c 'echo "side $0" >&2; echo "value $0"' $i)
  echo "body $i $y"
  if test $i = 3; then break; fi
done
# 値を読まない $( ) が終わってから、別の $( ) を起動する
a=$(seq 1 100000)
b=$(echo short)
echo "$b"
for x in $(seq 1 2); do echo "x $x"; done
echo end

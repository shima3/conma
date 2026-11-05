# 外部コマンド1つだけの $( ) は、OS のパイプから直接読む。値は改行（\n と \r\n）を除いた行の LazyList
x=$(seq 1 5)
echo ${x}
for i in ${x}; do echo "item $i"; done
y=$(/usr/bin/printf 'a\r\nb\nc')
for i in ${y}; do echo "[$i]"; done
z=$(/usr/bin/printf '')
echo "empty: ${z}."
echo $(seq 1 3)

# stdin：標準入力の要素を標準出力に出す。呼び出した関数と呼び出し側とで読み出し位置を共有する
function k () { stdin > (a . rest); stdin > (b); echo "a=$a b=$b rest=${rest}"; }
printf '1\n2\n3\n' | k
printf '1\n2\n3\n' > /tmp/consh.step6.txt
function h () { stdin > (first); stdin > (second . others); echo "$first $second ${others}"; }
h < /tmp/consh.step6.txt
rm /tmp/consh.step6.txt
echo $(printf 'k\nl\n' | stdin)
printf 'p\nq\n' | stdin | cat

# f > (a b . rest) & では、a、b は値ができるまで待つ遅延の値、rest は要素が届くたびに伸びる LazyList になる
function slow () { echo one; sleep 1; echo two; }
slow > (a b) &
echo "a=$a"
echo "b=$b"
printf 'x\ny\nz\n' > (p . r) &
echo "p=$p r=${r}"
# 値が足りなければ空文字列
echo only > (c d) &
echo "c=$c d=[$d]"

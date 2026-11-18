# seq 1 1000000 > (first . rest) で受け、for 文で rest を読む
seq 1 1000000 > (first . rest)
n=0; for x in $rest; do n=$x; done; echo $first $n

# seq 1 1000000 > (first . rest) で受け、for 文で rest を読む（read と $(cat) で）
set -u
{ read -r first; rest=$(cat); } < <(seq 1 1000000)
n=0; for x in $rest; do n=$x; done; echo $first $n

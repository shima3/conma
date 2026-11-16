# n=0; for i in $(seq 1 500000); do n=$i; done; echo $n（02 の 5 倍）
set -u
n=0; for i in $(seq 1 500000); do n=$i; done; echo $n

# echo $i を 50 万回（/dev/null へ）
set -u
for i in $(seq 1 500000); do echo $i; done > /dev/null

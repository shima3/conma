# echo $i を 1 万回（/dev/null へ）
set -u
for i in $(seq 1 10000); do echo $i; done > /dev/null

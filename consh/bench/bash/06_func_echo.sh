# 関数 f $i（echo "x$a"）を 3000 回（/dev/null へ）
set -u
f() { local a=$1; echo "x$a"; }
for i in $(seq 1 3000); do f $i; done > /dev/null

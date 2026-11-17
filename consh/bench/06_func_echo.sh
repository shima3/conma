# 関数 f $i（echo "x$a"）を 25 万回（/dev/null へ）
function f (a) { echo "x$a"; }
for i in $(seq 1 250000); do f $i; done > /dev/null

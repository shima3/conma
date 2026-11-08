# 関数 g $i（n=$x）を 5000 回
function g (x) { n=$x; }
for i in $(seq 1 5000); do g $i; done

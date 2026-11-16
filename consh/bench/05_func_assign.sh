# 関数 g $i（n=$x して echo $n）を 5000 回。値は > (n) で受ける
function g (x) { n=$x; echo $n; }
for i in $(seq 1 5000); do g $i > (n); done
echo $n

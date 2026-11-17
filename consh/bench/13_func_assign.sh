# 関数 g $i（n=$x して echo $n）を 10 万回（05 の 20 倍）。値は > (n) で受ける
function g (x) { n=$x; echo $n; }
for i in $(seq 1 100000); do g $i > (n); done
echo $n

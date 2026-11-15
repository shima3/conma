# 関数 g $i（n=$x）を 10 万回（05 の 20 倍）
function g (x) { n=$x; }
for i in $(seq 1 100000); do g $i; done
echo $n

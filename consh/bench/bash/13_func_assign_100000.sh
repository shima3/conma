# 関数 g $i（n=$x して echo $n）を 10 万回（05 の 20 倍）。値は > (n) の代わりに n=$( ) で受ける
set -u
g() { local x=$1; local n=$x; echo $n; }
for i in $(seq 1 100000); do n=$(g $i); done
echo $n

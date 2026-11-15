# 関数 g $i（n=$x して echo $n）を 5000 回。値は > (n) の代わりに n=$( ) で受ける
set -u
g() { local x=$1; local n=$x; echo $n; }
for i in $(seq 1 5000); do n=$(g $i); done
echo $n

# add $i x > (r) を 5 万回（08 の約 17 倍。> (r) は r=$( )）
set -u
add() { local a=$1 b=$2; echo $a$b; }
for i in $(seq 1 50000); do r=$(add $i x); done
echo $r

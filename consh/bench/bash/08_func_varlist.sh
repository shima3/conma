# add $i x > (r) を 3000 回（> (r) は r=$( )）
set -u
add() { local a=$1 b=$2; echo $a$b; }
for i in $(seq 1 3000); do r=$(add $i x); done
echo $r

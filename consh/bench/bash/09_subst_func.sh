# y=$(f $i) を 1000 回
set -u
f() { local a=$1; echo "x$a"; }
for i in $(seq 1 1000); do y=$(f $i); done
echo $y

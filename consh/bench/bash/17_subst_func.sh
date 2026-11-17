# y=$(f $i) を 3 万回
set -u
f() { local a=$1; echo "x$a"; }
for i in $(seq 1 30000); do y=$(f $i); done
echo $y

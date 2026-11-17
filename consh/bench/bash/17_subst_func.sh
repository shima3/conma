# y=$(f $i) を 2 万回
set -u
f() { local a=$1; echo "x$a"; }
for i in $(seq 1 20000); do y=$(f $i); done
echo $y

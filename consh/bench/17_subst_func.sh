# y=$(f $i) を 3 万回
function f (a) { echo "x$a"; }
for i in $(seq 1 30000); do y=$(f $i); done
echo $y

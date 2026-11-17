# add $i x > (r) を 5 万回
function add (a b) { echo $a$b; }
for i in $(seq 1 50000); do add $i x > (r); done
echo $r

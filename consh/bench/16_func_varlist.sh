# add $i x > (r) を 25 万回
function add (a b) { echo $a$b; }
for i in $(seq 1 250000); do add $i x > (r); done
echo $r

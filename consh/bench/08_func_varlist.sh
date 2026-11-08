# add $i x > (r) を 3000 回
function add (a b) { echo $a$b; }
for i in $(seq 1 3000); do add $i x > (r); done
echo $r

# seq 1 100000 | cnt（cnt は for x in $(stdin) で数える関数）
function cnt () { n=0; for x in $(stdin); do n=$x; done; echo $n; }
seq 1 100000 | cnt

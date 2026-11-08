# xs=$(seq 1 100) の入れ子の for 文で n=1 を 1 万回
xs=$(seq 1 100)
for i in ${xs}; do for j in ${xs}; do n=1; done; done
echo $n

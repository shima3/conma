# xs=$(seq 1 300) の入れ子の for 文で、if defined と関数 h $i $j（m=$a$b して echo $m）を 9 万回（03 と 04 を合わせて大きくしたもの）。値は > (m) で受ける
function h (a b) { m=$a$b; echo $m; }
xs=$(seq 1 300)
for i in ${xs}; do for j in ${xs}; do if defined m; then h $i $j > (m); else m=0; fi; done; done
echo $m

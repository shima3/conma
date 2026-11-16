# xs=$(seq 1 300) の入れ子の for 文で、if defined と関数 h $i $j（m=$a$b して echo $m）を 9 万回（03 と 04 を合わせて大きくしたもの）。値は > (m) の代わりに m=$( ) で受ける
set -u
h() { local a=$1 b=$2; local m=$a$b; echo $m; }
xs=$(seq 1 300)
for i in ${xs}; do for j in ${xs}; do if [ -n "${m+x}" ]; then m=$(h $i $j); else m=0; fi; done; done
echo $m

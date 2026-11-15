# xs=$(seq 1 300) の入れ子の for 文で、if defined と関数 h $i $j（m=$a$b）を 9 万回（03 と 04 を合わせて大きくしたもの）
set -u
h() { local a=$1 b=$2; local m=$a$b; }
xs=$(seq 1 300)
for i in ${xs}; do for j in ${xs}; do if [ -n "${m+x}" ]; then h $i $j; else m=0; fi; done; done
echo $m

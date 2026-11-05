# 読み終えた $( ) のパイプは閉じる（fd の上限 64 でも、何千回でも使える）。読まないままの出力も、外部コマンドが終われば閉じる
for i in $(seq 1 2000); do x=$(expr $i + 1); done
echo $x
for i in $(seq 1 200); do y=$(seq 1 20); done
echo ${y}

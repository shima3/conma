# seq 1 100000 > (first . rest)（read と $(cat) で）
set -u
{ read -r first; rest=$(cat); } < <(seq 1 100000)
echo $first

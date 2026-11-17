# seq 1 4000000 > (first . rest)（read と $(cat) で）
set -u
{ read -r first; rest=$(cat); } < <(seq 1 4000000)
echo $first

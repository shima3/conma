# seq 1 600000 | cnt（cnt は for x in $(cat) で数える関数）
set -u
cnt() { local n=0 x; for x in $(cat); do n=$x; done; echo $n; }
seq 1 600000 | cnt

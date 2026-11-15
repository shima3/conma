# 関数 g $i（n=$x）を 5000 回（ConSh と同じく、引数と代入は関数のローカル）
set -u
g() { local x=$1; local n=$x; }
for i in $(seq 1 5000); do g $i; done

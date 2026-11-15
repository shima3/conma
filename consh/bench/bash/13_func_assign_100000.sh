# 関数 g $i（n=$x）を 10 万回（05 の 20 倍。n は関数のローカルなので、ConSh と同じく最後の echo $n はエラー）
set -u
g() { local x=$1; local n=$x; }
for i in $(seq 1 100000); do g $i; done
echo $n

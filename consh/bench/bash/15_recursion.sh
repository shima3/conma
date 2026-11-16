# リストの先頭を除いて自分を呼ぶ関数 down（深さ 500）を 5 回。戻ると、先頭と下の結果をつなぐ
# （ConSh の down $rest > (r) を r=$( ) にすると、2500 回の fork の時間を測ることになるので、結果は大域変数 r で返す）
set -u
down() {
  local x=$1; shift
  if [ $# -gt 0 ]; then down "$@"; r="$x $r"; else r=$x; fi
}
xs=$(seq 1 500)
for i in $(seq 1 5); do down ${xs}; done
echo $r

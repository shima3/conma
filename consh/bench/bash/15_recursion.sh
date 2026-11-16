# リストの先頭を除いて自分を呼ぶ関数 down（深さ 1000）を 1000 回。戻ると、先頭と下の結果をつなぐ
# （ConSh の down ${rest} > (r) を r=$( ) にすると 100 万回 fork して遅すぎるので、結果は大域変数 r で返す）
set -u
down() {
  local x=$1; shift
  if [ $# -gt 0 ]; then down "$@"; r="$x $r"; else r=$x; fi
}
xs=$(seq 1 1000)
for i in $(seq 1 1000); do down ${xs}; done
echo $r

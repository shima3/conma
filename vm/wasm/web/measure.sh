#!/bin/bash
# canvas に線を 1 本描く 1 回の時間を、3 つの呼び方で測る（measure.se。docs/dev/PLATFORM-PLAN-ja.md の 3.4、段階 P3）。
# wasm 版の VM（Node.js）で、何もしない canvas（measure_host.js）に n 本ずつ描き、引数を作るだけの時間（base）を引いて、
# 1 本あたりのマイクロ秒を出す。描くこと自体の時間（ブラウザの canvas）は含まない。
#   vm/wasm/web/measure.sh [n] [回数]   （n は 50000、回数は 3。それぞれの中央値）
set -e -o pipefail
web="$(cd "$(dirname "$0")"; pwd -P)"
root="$(cd "$web/../../.."; pwd -P)"
n="${1:-50000}"
runs="${2:-3}"
tmp="$(mktemp -d /tmp/conma-measure.XXXXXX)"
trap 'rm -rf "$tmp"' EXIT
"$root/bin/includer" --include "$root/include/ext" --include "$root/include" "$web/measure.se" \
  | CONMA_EXT=1 "$root/bin/resolver" > "$tmp/measure.module.se"
# 1 回の秒数
one() {
  local s e
  s=$(python3 -c 'import time; print(time.time())')
  MEASURE_N="$n" CONMA_HOST="$web/measure_host.js" node "$root/vm/wasm/vm-node.js" "$tmp/measure.module.se" "$1" > /dev/null
  e=$(python3 -c 'import time; print(time.time())')
  python3 -c "print($e - $s)"
}
median() {
  local t=()
  for _ in $(seq "$runs"); do t+=("$(one "$1")"); done
  python3 -c "import statistics, sys; print(statistics.median(map(float, sys.argv[1:])))" "${t[@]}"
}
base=$(median base)
printf '%-6s %8.3f s\n' base "$base"
for m in host draw js; do
  t=$(median $m)
  python3 -c "print('%-6s %8.3f s  %8.2f µs a line' % ('$m', $t, ($t - $base) / $n * 1e6))"
done

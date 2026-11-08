#!/bin/bash
# test/ の下のすべての test.sh を実行する。diff の出力がなければ成功。
# VM は CONMA_VM で選ぶ（vm.sh を参照）。例: CONMA_VM=rust test/run_all.sh
# -j N を付けると、N 個ずつ並行に実行する（各テストは自分の一時ディレクトリを使う）。
# 出力はテストごとにまとめ、終わった後に、並行にしないときと同じ順に出す。例: test/run_all.sh -j 8
cd "$(dirname "$0")"
jobs=1
if [ "$1" = -j ]; then
  jobs=$2
  shift 2
fi
tests=(step*/test.sh module*/test.sh selfhost*/test.sh frontend/test.sh transpiled/test.sh)
if [ "$jobs" -le 1 ]; then
  for t in "${tests[@]}"
  do
    echo "== ${t%/test.sh}"
    bash $t
  done
  exit
fi
out="$(mktemp -d /tmp/conma-run_all.XXXXXX)" || exit 1
trap 'rm -rf "$out"' EXIT
printf '%s\n' "${tests[@]}" | xargs -P "$jobs" -I{} bash -c 't=$1; bash "$t" > "$2/${t%/test.sh}.out" 2>&1' _ {} "$out"
for t in "${tests[@]}"
do
  echo "== ${t%/test.sh}"
  cat "$out/${t%/test.sh}.out"
done

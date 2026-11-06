#!/bin/bash
# CONMA_SEED の種を変えて、テストを繰り返す。CONMA_SEED があると、VM のスケジューラは、
# 次に実行する VProc を、実行できる VProc の中から乱数で選ぶ（DESIGN.md の「Scheduling and Termination」）。
# VProc の実行の順に頼っている所があれば、違いと、その種を出す。違いがなければ何も出さない。
# エラーのメッセージの「at step N」は、全 VProc を通したステップ数で、順によって変わるので、
# diff の塊のうち、それだけが違うものは出さない。
# VM は CONMA_VM で選ぶ（既定は rust）。テストのディレクトリは、test/ からの相対パスで書く。
#   test/stress.sh [回数（既定 10）] [テストのディレクトリ ...]
#   test/stress.sh 20 step10 step17 step20
#   test/stress.sh 5 ../consh/test/step6        # ConSh（ConMa 版）のテスト
cd "$(dirname "$0")"
n=${1:-10}
shift
dirs=${*:-$(echo step*/ module*/ selfhost*/ | tr -d /)}
export CONMA_VM=${CONMA_VM:-rust}
filter="$(mktemp /tmp/conma-stress.XXXXXX)" || exit 1
trap 'rm -f "$filter"' EXIT
cat > "$filter" <<'PY'
# diff の出力から、「at step N」の数だけが違う塊を除く
import re, sys
def norm(line): return re.sub(r" at step \d+:", " at step N:", line[2:])
hunks, cur = [], None
for line in sys.stdin:
    if line[:1] in "<>-" and cur is not None:
        cur.append(line)
    else:
        cur = [line]
        hunks.append(cur)
for h in hunks:
    old = sorted(norm(l) for l in h if l.startswith("< "))
    new = sorted(norm(l) for l in h if l.startswith("> "))
    if (old or new) and old == new:
        continue
    sys.stdout.writelines(h)
PY
for seed in $(seq 1 $n); do
  for d in $dirs; do
    out=$(CONMA_SEED=$seed bash $d/test.sh 2>&1 | python3 -I "$filter")
    if [ -n "$out" ]; then
      echo "== $d seed=$seed"
      echo "$out" | head -20
    fi
  done
done

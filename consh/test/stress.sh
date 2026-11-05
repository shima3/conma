#!/bin/bash
# Rust 版（CONSH_IMPL=rust）で、並行に動くものを使うテストを、CONSH_JITTER の種を変えて繰り返す。
# CONSH_JITTER を付けると、Rust 版は、スレッドが出会う所（チャネル、書き込み、スレッドの始め、文の区切り）で
# 乱数で短く止まるか、ほかのスレッドに譲る。競合で結果が変わる所があれば、違いと、その種を出す。
# ケースの結果は同じ /tmp/consh.caseN.log に書くので、一度に 1 つずつ実行する。
#   test/stress.sh [回数（既定 10）] [テストのディレクトリ ...]
cd "$(dirname "$0")"
n=${1:-10}
shift
dirs=${*:-step2 step3 step4 step5 step6 step7 step8 step9 step10 step11 step12 step13 step14 step15 step16 examples}
for seed in $(seq 1 $n); do
  for d in $dirs; do
    out=$(CONSH_IMPL=rust CONSH_JITTER=$seed bash $d/test.sh 2>&1)
    if [ -n "$out" ]; then
      echo "== $d seed=$seed"
      echo "$out" | head -20
    fi
  done
done

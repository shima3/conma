#!/bin/bash
# test/ の下のすべての test.sh を順に実行する。diff の出力がなければ成功。
# VM は CONMA_VM で選ぶ（vm.sh を参照）。例: CONMA_VM=rust test/run_all.sh
cd "$(dirname "$0")"
for t in step*/test.sh module*/test.sh selfhost*/test.sh
do
  echo "== ${t%/test.sh}"
  bash $t
done

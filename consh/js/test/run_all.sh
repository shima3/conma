#!/bin/bash
# consh/js/test の下のすべての test.sh を実行する（wasm 版の VM。consh/js/build.sh で作っておく）。diff の出力がなければ成功
cd "$(dirname "$0")"
for t in */test.sh
do
  echo "== ${t%/test.sh}"
  bash $t
done

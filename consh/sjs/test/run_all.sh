#!/bin/bash
# consh/sjs/test の下のすべての test.sh を実行する（Rust 版の VM。vm で cargo build --release しておく）。diff の出力がなければ成功
cd "$(dirname "$0")"
for t in */test.sh
do
  echo "== ${t%/test.sh}"
  bash $t
done

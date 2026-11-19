#!/bin/bash
# conjs/test の下のすべてのステップを動かす（docs/dev/PLATFORM-PLAN-ja.md の 4.2 節）。diff の出力がなければ成功。
# VM は CONMA_VM（なし：Rust 版、wasm、python）。変換は Rust 版の VM（vm で cargo build --release しておく）
cd "$(dirname "$0")"
for t in step*/
do
  echo "== ${t%/}"
  bash run_case.sh ${t%/}
done

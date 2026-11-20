#!/bin/bash
# conjs/test の下のすべてのステップを動かす（docs/dev/PLATFORM-PLAN-ja.md の 4.2 節）。diff の出力がなければ成功。
# VM は CONMA_VM（なし：Rust 版、wasm、python）。変換は Rust 版の VM（vm で cargo build --release しておく）
cd "$(dirname "$0")"
for t in step*/
do
  echo "== ${t%/}"
  bash run_case.sh ${t%/}
done
# 待つ呼び出しのライブラリ（conjs/include/conjs_lib.se。4.4 節、段階 P5）が使うホストの関数と機能の名前が、docs/PRIMITIVES.md に
# あることを確かめる（知らない名前は黙って onError や偽になり、書き間違いと区別できないため。test/fast16 と同じ）
for name in $(grep -o '"[a-z]*\.[a-z_]*"' ../include/conjs_lib.se | tr -d '"' | grep -v '^fs\.' | sort -u) $(grep -o '__Feature_available__ "[a-z]*"' ../include/conjs_lib.se | sed 's/.*"\(.*\)"/\1/' | sort -u)
do
  grep -q "\`\"$name\"\`" ../../docs/PRIMITIVES.md || echo "$name is not in docs/PRIMITIVES.md"
done

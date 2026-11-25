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
# ホストの関数（docs/dev/CONJS-HOST-PLAN.md の段階 H3a）：wasm 版の VM では、step20 のホストの関数が、ホストの JavaScript で値を
# 返すことを確かめる（ホストの版が動かなくても、ConMa 版に替わってログは同じになるため）。case*.host.txt は、ホストの版で値を
# 得た関数の名前（CONJS_HOST_REPORT の出力を、LC_ALL=C の順に並べたもの。ロケールの順では、大文字と小文字の順が機械で
# 違う）。CONJS_HOST_STRICT で、変換器の作った JavaScript の誤りも出す。違いが step20 のものと分かるように見出しを出す
if [ "$CONMA_VM" = wasm ]; then
  echo "== step20 (host)"
  for c in step20/case*.con.js
  do
    CONJS_HOST_REPORT=1 CONJS_HOST_STRICT=1 ../conjs "$c" 2>&1 > /dev/null < /dev/null | grep '^conjs: host' | LC_ALL=C sort | diff "${c%.con.js}.host.txt" -
  done
fi

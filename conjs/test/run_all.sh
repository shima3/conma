#!/bin/bash
# conjs/test の下のすべてのステップを動かす（docs/dev/PLATFORM-PLAN-ja.md の 4.2 節）。diff の出力がなければ成功。
# VM は wasm 版（conjs/lang.conf の vm。段階 P10、2026-11-25 から。それまでは CONMA_VM で Rust 版、wasm 版、Python 版を選び、
# 3 つで流した）。vm/wasm/build.sh で作っておく。変換は、Rust 版の VM があればそれで行い、速い
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
if [ "${CONMA_VM:-wasm}" = wasm ]; then
  echo "== step20 (host)"
  for c in step20/case*.con.js
  do
    CONJS_HOST_REPORT=1 CONJS_HOST_STRICT=1 ../conjs "$c" 2>&1 > /dev/null < /dev/null | grep '^conjs: host' | LC_ALL=C sort | diff "${c%.con.js}.host.txt" -
  done
  # eval を止めたページ（docs/dev/CONJS-HOST-PLAN.md の 32 節、段階 H7）：Node.js が new Function を止めると、ホストの関数を
  # 作れず、ConMa 版だけで動くので、ログは同じになる。run_case.sh（bin/lang-test）で、case*.feed.py の入力も渡して流す。
  # CONJS_HOST_STRICT は消して流す（あると、作れなかったことを出し、ログと違う。ランタイムは、あるかだけを見るので、空にしても
  # 効かない。CONJS_HOST_STRICT=1 で run_all.sh を流しても通るように。M4 のレビュー、shima3/conma#87）
  echo "== step20 (no eval)"
  env -u CONJS_HOST_STRICT NODE_OPTIONS="${NODE_OPTIONS:+$NODE_OPTIONS }--disallow-code-generation-from-strings" bash run_case.sh step20
  # ConJS の async 関数（docs/dev/CONJS-HOST-PLAN.md の 34 節）：本体の console.log は、出力がパイプでも、プログラムの出力と順が
  # そろう（vm-node.js の asyncScope。Node.js の console は、macOS と Windows ではパイプに非同期に書く。M2 のレビュー、
  # shima3/conma#90）。bin/lang-test は出力をファイルに書くので、ここでパイプにつなぐ。new Function を止めると、ConMa 版が
  # ないので、始めで止まる
  echo "== step22 (pipe)"
  (cd step22 && ../../conjs case7.con.js 2> /dev/null | (sleep 1; cat) | diff <(grep -v -e '^exit:' -e '^host error:' case7.log) -)
  echo "== step22 (no eval)"
  (cd step22 && NODE_OPTIONS="${NODE_OPTIONS:+$NODE_OPTIONS }--disallow-code-generation-from-strings" ../../conjs case7.con.js 2>&1; echo "exit: $?") \
    | sed 's|of .*/step22/|of step22/|' \
    | diff - <(printf '%s\n' 'conjs: the async functions of step22/case7.con.js need JavaScript on the host: __JS_fn_compile__: Code generation from strings disallowed for this context' 'exit: 1')
  # コマンド行と Queue（docs/dev/CONJS-SHELL-PLAN.md の D15）：new Function を止めると、ConMa 版がないので、始めで止まる
  echo "== step25 (no eval)"
  for c in case7 case8
  do
    line=$(grep -n -m1 -e '^\$ ' -e 'new Queue' step25/$c.con.js | cut -d: -f1)
    (cd step25 && NODE_OPTIONS="${NODE_OPTIONS:+$NODE_OPTIONS }--disallow-code-generation-from-strings" ../../conjs $c.con.js 2>&1; echo "exit: $?") \
      | sed 's|(.*/step25/|(|' \
      | diff - <(printf '%s\n' "conjs: command lines and Queue need the JavaScript of the host ($c.con.js:$line): __JS_fn_compile__: Code generation from strings disallowed for this context" 'exit: 1')
  done
fi

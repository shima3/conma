#!/bin/bash
# canvas の見本のページ（canvas.html。docs/dev/PLATFORM-PLAN-ja.md の 3.4、段階 P3）に要るファイルを、ここに置く：
# wasm 版の VM（vm/wasm/build.sh でビルドする）とホストの conma.js、canvas のホストの関数（vm/wasm/host/canvas.js）、
# 見本の ConMa のプログラムのモジュール（canvas_demo.module.se）と、その ConJS 版（canvas_demo.con.js。段階 P3b）を
# 変換したモジュール（canvas_demo_conjs.module.se。変換に Rust 版の VM を使う）。入力の見本（stars.html。段階 P5c）の
# input.js（vm/wasm/host）と、stars.con.js を変換したモジュール（stars_conjs.module.se）も置く。置いたら、ここをサーバで見る：
#   vm/wasm/web/build.sh && python3 -m http.server -d vm/wasm/web -b 127.0.0.1 8766
# して、ブラウザで http://127.0.0.1:8766/canvas.html を開く（ConJS 版は canvas.html?conjs）
set -e -o pipefail
web="$(cd "$(dirname "$0")"; pwd -P)"
root="$(cd "$web/../../.."; pwd -P)"
"$root/vm/wasm/build.sh"
cp "$root/vm/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm" "$web/conma.wasm"
cp "$root/vm/wasm/conma.js" "$web/conma.js"
cp "$root/vm/wasm/host/canvas.js" "$web/canvas.js"
cp "$root/vm/wasm/host/input.js" "$web/input.js"
"$root/bin/includer" --include "$root/include/ext" --include "$root/include" "$web/canvas_demo.se" \
  | sed -e "s|\"$root/|\"conma/|g" | CONMA_EXT=1 "$root/bin/resolver" > "$web/canvas_demo.module.se"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/canvas-conjs.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
"$root/conjs/conjs" --emit "$web/canvas_demo.con.js" > "$tmp/canvas_demo_conjs.se"
"$root/bin/includer" --include "$root/conjs/include" --include "$root/sjs/include" --include "$root/include/ext" \
  --include "$root/include" "$tmp/canvas_demo_conjs.se" \
  | sed -e "s|\"$root/|\"conma/|g" -e "s|\"$tmp/|\"|g" | CONMA_EXT=1 "$root/bin/resolver" > "$web/canvas_demo_conjs.module.se"
# 入力を待つ対話の見本（stars.html と stars.con.js。4.6 節、段階 P5c）も、同じ形でモジュールにする
"$root/conjs/conjs" --emit "$web/stars.con.js" > "$tmp/stars_conjs.se"
"$root/bin/includer" --include "$root/conjs/include" --include "$root/sjs/include" --include "$root/include/ext" \
  --include "$root/include" "$tmp/stars_conjs.se" \
  | sed -e "s|\"$root/|\"conma/|g" -e "s|\"$tmp/|\"|g" | CONMA_EXT=1 "$root/bin/resolver" > "$web/stars_conjs.module.se"

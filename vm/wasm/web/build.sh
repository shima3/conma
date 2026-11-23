#!/bin/bash
# canvas の見本のページ（canvas.html。docs/dev/PLATFORM-PLAN-ja.md の 3.4、段階 P3）に要るファイルを、ここに置く：
# wasm 版の VM（vm/wasm/build.sh でビルドする）とホストの conma.js、canvas のホストの関数（vm/wasm/host/canvas.js）、
# 見本の ConMa のプログラムのモジュール（canvas_demo.module.se）。ConJS の見本（canvas_demo.con.js と stars.con.js の
# ページ）は、2026-11-23 から conjs/web にある（conjs/web/build.sh）。置いたら、ここをサーバで見る：
#   vm/wasm/web/build.sh && python3 -m http.server -d vm/wasm/web -b 127.0.0.1 8766
# して、ブラウザで http://127.0.0.1:8766/canvas.html を開く
set -e -o pipefail
web="$(cd "$(dirname "$0")"; pwd -P)"
root="$(cd "$web/../../.."; pwd -P)"
"$root/vm/wasm/build.sh"
cp "$root/vm/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm" "$web/conma.wasm"
cp "$root/vm/wasm/conma.js" "$web/conma.js"
cp "$root/vm/wasm/host/canvas.js" "$web/canvas.js"
"$root/bin/includer" --include "$root/include/ext" --include "$root/include" "$web/canvas_demo.se" \
  | sed -e "s|\"$root/|\"conma/|g" | CONMA_EXT=1 "$root/bin/resolver" > "$web/canvas_demo.module.se"

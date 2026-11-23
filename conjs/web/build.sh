#!/bin/bash
# ConJS のブラウザの見本のページ（canvas.html と stars.html）に要るファイルを、ここに置く：wasm 版の VM（vm/wasm/build.sh で
# ビルドする）とホストの conma.js、canvas と入力のホストの関数（vm/wasm/host の canvas.js と input.js）、見本の ConJS の
# プログラム（canvas_demo.con.js。docs/dev/PLATFORM-PLAN-ja.md の 4.2 節、段階 P3b。stars.con.js。4.6 節、段階 P5c。端末用は conjs/examples/stars-term.con.js）を
# 変換したモジュール（canvas_demo_conjs.module.se、stars_conjs.module.se。変換に Rust 版の VM を使う）。
# 2026-11-23 までは vm/wasm/web にあった（ConMa の見本 canvas_demo.se は、そこに残る）。置いたら、ここをサーバで見る：
#   conjs/web/build.sh && python3 -m http.server -d conjs/web -b 127.0.0.1 8768
# して、ブラウザで http://127.0.0.1:8768/stars.html か canvas.html を開く
set -e -o pipefail
web="$(cd "$(dirname "$0")"; pwd -P)"
root="$(cd "$web/../.."; pwd -P)"
"$root/vm/wasm/build.sh"
cp "$root/vm/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm" "$web/conma.wasm"
cp "$root/vm/wasm/conma.js" "$web/conma.js"
cp "$root/vm/wasm/host/canvas.js" "$web/canvas.js"
cp "$root/vm/wasm/host/input.js" "$web/input.js"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/conjs-web.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
# 見本の NAME.con.js を、変換して NAME_conjs.module.se にする
for name in canvas_demo stars; do
  "$root/conjs/conjs" --emit "$web/$name.con.js" > "$tmp/${name}_conjs.se"
  "$root/bin/includer" --include "$root/conjs/include" --include "$root/sjs/include" --include "$root/include/ext" \
    --include "$root/include" "$tmp/${name}_conjs.se" \
    | sed -e "s|\"$root/|\"conma/|g" -e "s|\"$tmp/|\"|g" | CONMA_EXT=1 "$root/bin/resolver" > "$web/${name}_conjs.module.se"
done

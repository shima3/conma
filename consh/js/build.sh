#!/bin/bash
# ConMa の VM を wasm32 でビルドし（consh/js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm）、
# ブラウザの画面（web/）に、その wasm と ConSh のモジュール（consh.module.se）を置く。
# rustup target add wasm32-unknown-unknown で、wasm32 のターゲットを入れておく。
# モジュールは bin/consh と同じく bin/includer と bin/resolver で作り、ソースが変わったときだけ作り直す
set -e
js="$(cd "$(dirname "$0")"; pwd)"
consh="${js%/*}"
root="${consh%/*}"
(cd "$js/wasm" && cargo build --release)
cp "$js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm" "$js/web/conma.wasm"
cp "$js/conma.js" "$js/web/conma.js"
module="$js/web/consh.module.se"
if [ ! -s "$module" ] || [ -n "$(find "$root/include" "$consh/include" -name '*.se' -newer "$module")" ]; then
  ast_tmp=$(mktemp "$js/web/consh.ast.XXXXXX")
  module_tmp=$(mktemp "$module.XXXXXX")
  trap 'rm -f "$ast_tmp" "$module_tmp"' EXIT
  "$root/bin/includer" --include "$root/include" --include "$consh/include" "$consh/include/consh_main.se" \
    | sed -e 's/"\/.*\/conma/"conma/' > "$ast_tmp"
  "$root/bin/resolver" < "$ast_tmp" > "$module_tmp"
  mv -f "$module_tmp" "$module"
fi

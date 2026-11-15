#!/bin/bash
# ConMa の VM を wasm32 でビルドし（vm/wasm/build.sh）、
# ブラウザの画面（web/）に、その wasm とホストの conma.js（vm/wasm）、ConSh のモジュール（consh.module.se）を置く。
# rustup target add wasm32-unknown-unknown で、wasm32 のターゲットを入れておく。
# モジュールは bin/consh と同じく bin/includer と bin/resolver で作り、ソースかフロントエンド（bin/）が変わったときだけ作り直す
set -e
js="$(cd "$(dirname "$0")"; pwd)"
consh="${js%/*}"
root="${consh%/*}"
"$root/vm/wasm/build.sh"
cp "$root/vm/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm" "$js/web/conma.wasm"
cp "$root/vm/wasm/conma.js" "$js/web/conma.js"
# (( )) の SJS のモジュールを、ページの中で作るための ConMa のフロントエンド（web/consh-core.js の buildModule）
cp "$root/vm/frontend.module.se" "$js/web/frontend.module.se"
module="$js/web/consh.module.se"
# ConSh は最適化版（consh/include と標準ライブラリの include/ext と include。consh/include/README-ja.md）
main="$consh/include/consh_main.se"
# （フォルダも見るのは、ファイルを足したり消したり移したりすると、フォルダの時刻が変わるから）
if [ ! -s "$module" ] || [ -n "$(find "$root/include" "$consh/include" \( -name '*.se' -o -type d \) -newer "$module")" ] ||
   [ -n "$(find "$root/bin" -type f -newer "$module")" ]; then
  ast_tmp=$(mktemp "$js/web/consh.ast.XXXXXX")
  module_tmp=$(mktemp "$module.XXXXXX")
  trap 'rm -f "$ast_tmp" "$module_tmp"' EXIT
  "$root/bin/includer" --include "$consh/include" --include "$root/include/ext" --include "$root/include" "$main" \
    | sed -e 's/"\/.*\/conma/"conma/' > "$ast_tmp"
  "$root/bin/resolver" < "$ast_tmp" > "$module_tmp"
  mv -f "$module_tmp" "$module"
fi

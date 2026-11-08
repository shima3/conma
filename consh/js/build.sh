#!/bin/bash
# ConMa の VM を wasm32 でビルドし（consh/js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm）、
# ブラウザの画面（web/）に、その wasm と ConSh のモジュール（consh.module.se）を置く。
# rustup target add wasm32-unknown-unknown で、wasm32 のターゲットを入れておく。
# モジュールは bin/consh と同じく bin/includer と bin/resolver で作り、ソースかフロントエンド（bin/）が変わったときだけ作り直す
set -e
js="$(cd "$(dirname "$0")"; pwd)"
consh="${js%/*}"
root="${consh%/*}"
# cargo は PATH から探す。なければ rustup の既定の場所（~/.cargo/bin）と、Homebrew の rustup の場所
# （keg-only で PATH に入らない。/opt/homebrew/opt/rustup/bin、Intel の Mac では /usr/local/opt/rustup/bin）を使う
cargo="$(command -v cargo || true)"
if [ -z "$cargo" ]; then
  for d in "$HOME/.cargo/bin" /opt/homebrew/opt/rustup/bin /usr/local/opt/rustup/bin; do
    if [ -x "$d/cargo" ]; then
      cargo="$d/cargo"
      export PATH="$d:$PATH"  # cargo が呼ぶ rustc なども
      break
    fi
  done
fi
if [ -z "$cargo" ]; then
  echo "build.sh: cargo is not found (install Rust with rustup, or add the directory of cargo to PATH)" >&2
  exit 1
fi
(cd "$js/wasm" && "$cargo" build --release)
cp "$js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm" "$js/web/conma.wasm"
cp "$js/conma.js" "$js/web/conma.js"
module="$js/web/consh.module.se"
# ConSh は高速版（consh/include と標準ライブラリの include。consh/include/README-ja.md）
main="$consh/include/consh_main.se"
# （フォルダも見るのは、ファイルを足したり消したり移したりすると、フォルダの時刻が変わるから）
if [ ! -s "$module" ] || [ -n "$(find "$root/include" "$consh/include" \( -name '*.se' -o -type d \) -newer "$module")" ] ||
   [ -n "$(find "$root/bin" -type f -newer "$module")" ]; then
  ast_tmp=$(mktemp "$js/web/consh.ast.XXXXXX")
  module_tmp=$(mktemp "$module.XXXXXX")
  trap 'rm -f "$ast_tmp" "$module_tmp"' EXIT
  "$root/bin/includer" --include "$consh/include" --include "$root/include" "$main" \
    | sed -e 's/"\/.*\/conma/"conma/' > "$ast_tmp"
  "$root/bin/resolver" < "$ast_tmp" > "$module_tmp"
  mv -f "$module_tmp" "$module"
fi

#!/bin/bash
# ConMa の VM を wasm32 でビルドする（vm/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm）。
# vm-wasm.sh（bin/vm の CONMA_VM=wasm）が使う。ブラウザ版の ConSh は consh/js/build.sh で作り、それがこれを呼ぶ。
# rustup target add wasm32-unknown-unknown で、wasm32 のターゲットを入れておく
set -e
dir="$(cd "$(dirname "$0")"; pwd)"
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
(cd "$dir" && "$cargo" build --release)

#!/bin/bash
# ConMa の VM を wasm32 でビルドする（consh/js/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm）。
# rustup target add wasm32-unknown-unknown で、wasm32 のターゲットを入れておく。
set -e
cd "$(dirname "$0")/wasm"
cargo build --release

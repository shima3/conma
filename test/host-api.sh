#!/bin/bash
# test/host-api.sh：使う人がホストの関数を足す口のテスト（docs/HOST-ja.md、docs/dev/PLATFORM-PLAN-ja.md の 4.9 節、段階 P7）。
# Rust の API で関数を足した vm/examples/host_demo.rs を作り（cargo build --release --example host_demo。Full Test の手順では
# example が作られないので、ここで作る）、conjs/examples/host_demo.con.js を動かす。同じ関数を JavaScript で書いた
# conjs/examples/host_demo.host.js を CONMA_HOST にして、wasm 版の VM（Node.js）でも動かし、同じログになることを確かめる
# （wasm 版の VM がないか、node がなければ飛ばす）。ほかに、Rust の API だけのこと（test/host-api/rust.con.js）と、関数を
# 足さない VM では host.available("demo") が偽になること。diff の出力がなければ成功。ConJS の既定の VM は wasm 版なので
# （段階 P10）、Rust 版で動かす所は CONMA_VM=rust を付ける（CONMA_RUNNER は Rust 版でだけ効く）
root="$(cd "$(dirname "$0")/.."; pwd)"
dir="$root/test/host-api"
export PATH="$HOME/.cargo/bin:$PATH"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/host-api.XXXXXX")" || exit 1
trap 'rm -rf "$tmp"' EXIT
(cd "$root/vm" && cargo build --release --example host_demo -q 2> "$tmp/build.err") || { cat "$tmp/build.err"; exit 1; }
demo="$root/vm/target/release/examples/host_demo"
# run ログ 環境 ... -- ファイル
run() {
  local log="$1"; shift
  env "$@" > "$tmp/out" 2>&1 < /dev/null
  echo "exit: $?" >> "$tmp/out"
  diff "$dir/$log" "$tmp/out"
}
echo "== rust"
run demo.log CONMA_VM=rust CONMA_RUNNER="$demo" "$root/bin/lang" conjs "$root/conjs/examples/host_demo.con.js"
run rust.log CONMA_VM=rust CONMA_RUNNER="$demo" "$root/bin/lang" conjs "$dir/rust.con.js"
run nodemo.log CONMA_VM=rust "$root/bin/lang" conjs "$root/conjs/examples/host_demo.con.js"
if command -v node > /dev/null && [ -f "$root/vm/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm" ]; then
  echo "== wasm"
  run demo.log CONMA_VM=wasm CONMA_HOST="$root/conjs/examples/host_demo.host.js" "$root/bin/lang" conjs "$root/conjs/examples/host_demo.con.js"
  run nodemo.log CONMA_VM=wasm "$root/bin/lang" conjs "$root/conjs/examples/host_demo.con.js"
else
  echo "== wasm（wasm 版の VM か node がないので飛ばした）"
fi

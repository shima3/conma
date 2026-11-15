#!/bin/bash
# vm/conma2rust/build.sh MODULE.module.se ...
# Transpile the modules to Rust (conma2rust.py, the ConMa-to-Rust transpiler)
# and build the VM with that code in
# vm/target-transpiled (CONMA_VM=transpiled in bin/vm). The modules are files made
# by bin/resolver; their transpiled code is used only for files with the same text.
set -e
vm="$(cd "$(dirname "$0")/.."; pwd)"
mkdir -p "$vm/target-transpiled"
# cargo は、自分でディレクトリを作るときだけ CACHEDIR.TAG（バックアップなどの道具が除くための印）を書くので、
# 先に作ったここには同じものを書いておく
[ -f "$vm/target-transpiled/CACHEDIR.TAG" ] || printf '%s\n' 'Signature: 8a477f597d28d172789f06886806bc55' \
  '# This file is a cache directory tag created by cargo.' \
  '# For information about cache directory tags see https://bford.info/cachedir/' > "$vm/target-transpiled/CACHEDIR.TAG"
out="$vm/target-transpiled/transpiled_gen.rs"
python3 "$vm/conma2rust/conma2rust.py" "$out" "$@"
cd "$vm"
CONMA_TRANSPILED="$out" cargo build --release --target-dir target-transpiled

#!/bin/bash
# vm/conma2rust/build.sh MODULE.module.se ...
# Transpile the modules to Rust (conma2rust.py, the ConMa-to-Rust transpiler)
# and build the VM with that code in
# vm/target-transpiled (CONMA_VM=transpiled in test/vm.sh). The modules are files made
# by bin/resolver; their transpiled code is used only for files with the same text.
set -e
vm="$(cd "$(dirname "$0")/.."; pwd)"
mkdir -p "$vm/target-transpiled"
out="$vm/target-transpiled/transpiled_gen.rs"
python3 "$vm/conma2rust/conma2rust.py" "$out" "$@"
cd "$vm"
CONMA_TRANSPILED="$out" cargo build --release --target-dir target-transpiled

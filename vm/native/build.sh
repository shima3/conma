#!/bin/bash
# vm/native/build.sh MODULE.module.se ...
# Make native code for the modules (translate.py) and build the VM with it in
# vm/target-native (CONMA_VM=native in test/vm.sh). The modules are files made
# by bin/resolver; their native code is used only for files with the same text.
set -e
vm="$(cd "$(dirname "$0")/.."; pwd)"
mkdir -p "$vm/target-native"
out="$vm/target-native/native_gen.rs"
python3 "$vm/native/translate.py" "$out" "$@"
cd "$vm"
CONMA_NATIVE="$out" cargo build --release --target-dir target-native

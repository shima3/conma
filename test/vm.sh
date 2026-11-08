#!/bin/bash
# ConMa の VM を起動する。引数は vproc_run.py と同じ（[--trace] [--main] <module_file> <func_name | arg ...>）。
# 環境変数 CONMA_VM が rust なら Rust 版（vm/target/release/conma）、transpiled なら ConMa から Rust への変換器
# （vm/conma2rust）で変換したコードを組み込んだ Rust 版
# （vm/conma2rust/build.sh で作る vm/target-transpiled/release/conma）、wasm なら wasm32 版を JavaScriptCore で動かす
# （consh/js/vm-wasm.sh。consh/js/build.sh で作る）、それ以外（既定）は Python 版（vproc_run.py）
dir="$(cd "${0%/*}"; pwd)"
case "${CONMA_VM:-python}" in
  rust) exec "$dir/../vm/target/release/conma" "$@" ;;
  wasm) exec "$dir/../consh/js/vm-wasm.sh" "$@" ;;
  transpiled) exec "$dir/../vm/target-transpiled/release/conma" "$@" ;;
  *)    exec python3 "$dir/vproc_run.py" "$@" ;;
esac

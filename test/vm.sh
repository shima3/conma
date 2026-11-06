#!/bin/bash
# ConMa の VM を起動する。引数は vproc_run.py と同じ（[--trace] [--main] <module_file> <func_name | arg ...>）。
# 環境変数 CONMA_VM が rust なら Rust 版（vm/target/release/conma）、native ならネイティブコード付きの Rust 版
# （vm/native/build.sh で作る vm/target-native/release/conma）、それ以外（既定）は Python 版（vproc_run.py）
dir="$(cd "${0%/*}"; pwd)"
case "${CONMA_VM:-python}" in
  rust) exec "$dir/../vm/target/release/conma" "$@" ;;
  native) exec "$dir/../vm/target-native/release/conma" "$@" ;;
  *)    exec python3 "$dir/vproc_run.py" "$@" ;;
esac

#!/bin/bash
# Python 版と Rust 版の VM を同じ引数で --trace 付きで実行し、トレースと出力と終了コードを比べる。
# 違いがなければ何も出さない。引数は vproc_run.py と同じ（--trace は付けなくてよい）
#   test/vm_compare.sh /tmp/test.module.se case1
dir="$(cd "${0%/*}"; pwd)"
tmp=$(mktemp -d /tmp/vm_compare.XXXXXX)
trap 'rm -rf "$tmp"' EXIT
for vm in python rust
do
  CONMA_VM=$vm PYTHONUNBUFFERED=1 "$dir/vm.sh" --trace "$@" < /dev/null > "$tmp/$vm.out" 2> "$tmp/$vm.trace"
  echo "exit: $?" >> "$tmp/$vm.out"
done
diff "$tmp/python.out" "$tmp/rust.out"
diff "$tmp/python.trace" "$tmp/rust.trace" | head -40

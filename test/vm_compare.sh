#!/bin/bash
# Python 版と Rust 版の VM を同じ引数で実行し、出力と終了コード、および --trace のトレースを比べる。
# 違いがなければ何も出さない。引数は vproc_run.py と同じ（--trace は付けなくてよい）
#   test/vm_compare.sh /tmp/test.module.se case1
# トレースは 1 ステップごとに VProc 全体を書くので、大きな値を持つプログラムでは非常に大きくなる。
# そのため、トレースは先頭の VM_COMPARE_LIMIT バイト（既定 100MB）だけを比べる
dir="$(cd "${0%/*}"; pwd)"
limit=${VM_COMPARE_LIMIT:-100000000}
tmp=$(mktemp -d /tmp/vm_compare.XXXXXX)
trap 'rm -rf "$tmp"' EXIT
for vm in python rust
do
  CONMA_VM=$vm PYTHONUNBUFFERED=1 "$dir/vm.sh" "$@" < /dev/null > "$tmp/$vm.out" 2>&1
  echo "exit: $?" >> "$tmp/$vm.out"
  CONMA_VM=$vm PYTHONUNBUFFERED=1 "$dir/vm.sh" --trace "$@" < /dev/null 2>&1 > /dev/null | head -c $limit > "$tmp/$vm.trace"
done
diff "$tmp/python.out" "$tmp/rust.out"
cmp -s "$tmp/python.trace" "$tmp/rust.trace" || diff "$tmp/python.trace" "$tmp/rust.trace" | head -40

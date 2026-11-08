#!/bin/bash
# bench/build.sh: build the transpiled VM (vm/target-transpiled) with the transpiled code of every benchmark
# (and of ConSh, /tmp/consh.module.se and /tmp/consh.reference.module.se, the fast and reference versions,
# if they are there, so that ConSh keeps its transpiled code).
set -e
bench="$(cd "$(dirname "$0")"; pwd)"
tmp="$(mktemp -d /tmp/conma-bench.XXXXXX)"
trap 'rm -rf "$tmp"' EXIT
modules=()
for f in "$bench"/*/*.se; do
  n=$(basename "${f%.se}")
  "$bench/module.sh" "$f" "$tmp/$n.module.se"
  modules+=("$tmp/$n.module.se")
done
[ -s /tmp/consh.module.se ] && modules+=(/tmp/consh.module.se)
[ -s /tmp/consh.reference.module.se ] && modules+=(/tmp/consh.reference.module.se)
"$bench/../vm/conma2rust/build.sh" "${modules[@]}"

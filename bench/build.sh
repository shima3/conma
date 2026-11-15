#!/bin/bash
# bench/build.sh: build the transpiled VM (vm/target-transpiled) with the transpiled code of every benchmark,
# of the programs of test/transpiled (whose test then tests that code), and of ConSh (consh.module.se, the
# fast version, in the directory where consh/bin/consh makes it, CONSH_MODULE_DIR or bin/cache-dir, if it is
# there, so that ConSh keeps its transpiled code). With CONMA_BENCH_REFERENCE set to a non-empty value, also of
# the reference version of ConSh (consh.reference.module.se), to compare the two versions on the transpiled VM;
# the build then takes about twice as long.
set -e
bench="$(cd "$(dirname "$0")"; pwd)"
tmp="$(mktemp -d /tmp/conma-bench.XXXXXX)"
trap 'rm -rf "$tmp"' EXIT
modules=()
root="$(cd "$bench/.."; pwd)"
for f in "$bench"/*/*.se "$root"/test/transpiled/*.se; do
  n=$(basename "${f%.se}")
  "$bench/module.sh" "$f" "$tmp/$n.module.se"
  modules+=("$tmp/$n.module.se")
done
if [ -n "$CONSH_MODULE_DIR" ]; then consh_dir="$CONSH_MODULE_DIR"; else consh_dir="$("$root/bin/cache-dir")"; fi
[ -s "$consh_dir/consh.module.se" ] && modules+=("$consh_dir/consh.module.se")
[ -n "$CONMA_BENCH_REFERENCE" ] && [ -s "$consh_dir/consh.reference.module.se" ] && modules+=("$consh_dir/consh.reference.module.se")
"$bench/../vm/conma2rust/build.sh" "${modules[@]}"

#!/bin/bash
# bench/build.sh: build the transpiled VM (vm/target-transpiled) with the transpiled code of every benchmark,
# of the programs of test/transpiled (whose test then tests that code), and of ConSh (/tmp/consh.module.se, the
# fast version, if it is there, so that ConSh keeps its transpiled code). With CONMA_BENCH_REFERENCE set to a
# non-empty value, also of the reference version of ConSh (/tmp/consh.reference.module.se), to compare the two
# versions on the transpiled VM; the build then takes about twice as long.
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
[ -s /tmp/consh.module.se ] && modules+=(/tmp/consh.module.se)
[ -n "$CONMA_BENCH_REFERENCE" ] && [ -s /tmp/consh.reference.module.se ] && modules+=(/tmp/consh.reference.module.se)
"$bench/../vm/conma2rust/build.sh" "${modules[@]}"

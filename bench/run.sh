#!/bin/bash
# bench/run.sh NAME ARG...: run benchmark NAME (NAME/NAME.py and NAME/NAME.se) with CPython and the transpiled
# VM (built by build.sh), check that they print the same, and print the best of 3 times (user + sys seconds)
# of each. With CONMA_BENCH_RUST=1, the Rust VM too.
bench="$(cd "$(dirname "$0")"; pwd)"
root="${bench%/*}"
name=$1; shift
tmp="$(mktemp -d /tmp/conma-bench.XXXXXX)"
trap 'rm -rf "$tmp"' EXIT
"$bench/module.sh" "$bench/$name/$name.se" "$tmp/$name.module.se" || exit 1
used=$(CONMA_TRANSPILED_REPORT=1 "$root/vm/target-transpiled/release/conma" --main "$tmp/$name.module.se" "$@" 2>&1 >/dev/null | grep -c "transpiled code: used")
[ "$used" -ge 1 ] || echo "note: the transpiled VM has no transpiled code for $name (run bench/build.sh)"
run() {  # label command...
  "${@:2}" > "$tmp/$1.out" 2>&1 || { echo "$1 failed:"; cat "$tmp/$1.out"; exit 1; }
  local best=
  for i in 1 2 3; do
    t=$( { TIMEFORMAT='%U %S'; time "${@:2}" > /dev/null 2>&1; } 2>&1 | awk '{print $1 + $2}')
    if [ -z "$best" ] || awk "BEGIN{exit !($t < $best)}"; then best=$t; fi
  done
  printf '%-12s %6.2f s\n' "$1" "$best"
}
run cpython python3 "$bench/$name/$name.py" "$@"
[ -n "$CONMA_BENCH_RUST" ] && run rust "$root/vm/target/release/conma" --main "$tmp/$name.module.se" "$@"
run transpiled "$root/vm/target-transpiled/release/conma" --main "$tmp/$name.module.se" "$@"
for vm in transpiled ${CONMA_BENCH_RUST:+rust}; do
  cmp -s "$tmp/cpython.out" "$tmp/$vm.out" || { echo "$vm prints otherwise than cpython:"; diff "$tmp/cpython.out" "$tmp/$vm.out"; }
done

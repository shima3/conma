#!/bin/bash
# conjs/bench/run.sh NAME [N [M]]: measure the benchmark NAME.con.js of ConJS (2026-11-24; conjs/bench/README-ja.md).
# It runs the same program in Node.js (NAME.node.js when it is there, as for tasks, which uses spawn and join; otherwise
# NAME.con.js itself, which is JavaScript too), and with conjs/conjs on the Rust VM and on the wasm VM (CONMA_VM=wasm;
# Node.js), and prints the time of each: user + sys seconds, the least of 3 runs. conjs/conjs translates the program and
# joins it to the runtime each time, so the times of ConJS include that (the translator and the runtime are made once
# before, as conjs/conjs --prepare does). N_ARG in the program is replaced by the size: N for Node.js, and M (N when it is
# not given) for the slow columns, the two VMs, marked * (docs/dev/CONTRIBUTING.md, Benchmarks: the fastest version about
# one second, a slower one not over a minute). Without N, the size of each NAME below (Apple M4, 2026-11-24). It also
# checks that the VMs print what Node.js prints at size M. startup has no N_ARG and is run as it is.
bench="$(cd "$(dirname "$0")"; pwd)"
conjs="${bench%/*}/conjs"
name=$1 n=$2 m=${3:-$2}
if [ -z "$n" ]; then
  case "$name" in
    startup) n=1 m=1 ;;
    strings) n=3000000 m=180000 ;;
    dict) n=15000000 m=600000 ;;
    hof) n=350000 m=6000 ;;
    json) n=200000 m=4500 ;;
    exceptions) n=2000000 m=1200000 ;;
    tasks) n=1600000 m=60000 ;;
    nbody_class) n=20000000 m=27000 ;;
    *) echo "usage: conjs/bench/run.sh NAME [N [M]] (NAME: startup, strings, dict, hof, json, exceptions, tasks, nbody_class)" >&2; exit 2 ;;
  esac
fi
[ -f "$bench/$name.con.js" ] || { echo "run.sh: no $name.con.js" >&2; exit 2; }
node="$bench/$name.con.js"
[ -f "$bench/$name.node.js" ] && node="$bench/$name.node.js"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/conjs-bench.XXXXXX")" || exit 1
trap 'rm -rf "$tmp"' EXIT
sed "s/N_ARG/$n/" "$node" > "$tmp/fast.js"
sed "s/N_ARG/$m/" "$node" > "$tmp/slow.js"
sed "s/N_ARG/$m/" "$bench/$name.con.js" > "$tmp/slow.con.js"
best() {  # command...: the least of 3 runs, user + sys seconds
  local b= t i
  for i in 1 2 3; do
    t=$( { TIMEFORMAT='%U %S'; time "$@" > /dev/null 2>&1; } 2>&1 | awk '{print $1 + $2}')
    if [ -z "$b" ] || awk "BEGIN{exit !($t < $b)}"; then b=$t; fi
  done
  echo "$b"
}
"$conjs" --prepare > /dev/null || exit 1
CONMA_VM=wasm "$conjs" --prepare > /dev/null || exit 1
node "$tmp/slow.js" > "$tmp/node.out" 2>&1
for vm in rust wasm; do
  if [ $vm = rust ]; then "$conjs" "$tmp/slow.con.js" > "$tmp/$vm.out" 2>&1
  else CONMA_VM=wasm "$conjs" "$tmp/slow.con.js" > "$tmp/$vm.out" 2>&1; fi
  cmp -s "$tmp/node.out" "$tmp/$vm.out" || { echo "ConJS on the $vm VM prints otherwise than Node.js:"; diff "$tmp/node.out" "$tmp/$vm.out"; }
done
if [ "$m" = "$n" ]; then echo "== $name $n"; else echo "== $name $n (the slow columns, marked *: $m)"; fi
printf '%-16s %s s\n' "Node.js" "$(best node "$tmp/fast.js")"
printf '%-16s %s s\n' "Rust VM *" "$(best "$conjs" "$tmp/slow.con.js")"
printf '%-16s %s s\n' "wasm VM *" "$(best env CONMA_VM=wasm "$conjs" "$tmp/slow.con.js")"

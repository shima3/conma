#!/bin/bash
# conjs/bench/run.sh NAME [N [M]]: measure the benchmark NAME.con.js of ConJS (2026-11-24; conjs/bench/README-ja.md).
# It runs the same program in Node.js (NAME.node.js when it is there, as for tasks, which uses spawn and join; otherwise
# NAME.con.js itself, which is JavaScript too), and with conjs/conjs on the wasm VM (Node.js; the only VM of ConJS from
# stage P10, 2026-11-25: until then also on the Rust VM), and prints the time of each: user + sys seconds (real time on Windows), the least of 3 runs. conjs/conjs translates the program and
# joins it to the runtime each time, so the times of ConJS include that (the translator and the runtime are made once
# before, as conjs/conjs --prepare does). N_ARG in the program is replaced by the size: N for Node.js, and M (N when it is
# not given) for the slow column, the wasm VM, marked * (docs/dev/CONTRIBUTING.md, Benchmarks: the fastest version about
# one second, a slower one not over a minute). Without N, the size of each NAME below (Apple M4, 2026-11-24). It also
# checks that the wasm VM prints what Node.js prints at size M. startup has no N_ARG and is run as it is.
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
# On Windows (Git Bash, MSYS), time does not count the user and sys time of the native programs of Windows (node.exe,
# conma.exe): there the real time is measured instead, and the output says so (S5's review of shima3/conma#71)
clock='%U %S' what='user + sys'
case "$(uname -o 2> /dev/null)" in Msys|Cygwin) clock='%R 0' what='real time (Windows: user + sys misses native programs)' ;; esac
best() {  # command...: the least of 3 runs, in seconds of $what
  local b= t i
  for i in 1 2 3; do
    t=$( { TIMEFORMAT="$clock"; time "$@" > /dev/null 2>&1; } 2>&1 | awk '{print $1 + $2}')
    if [ -z "$b" ] || awk "BEGIN{exit !($t < $b)}"; then b=$t; fi
  done
  echo "$b"
}
"$conjs" --prepare > /dev/null || exit 1
node "$tmp/slow.js" > "$tmp/node.out" 2>&1
"$conjs" "$tmp/slow.con.js" > "$tmp/wasm.out" 2>&1
cmp -s "$tmp/node.out" "$tmp/wasm.out" || { echo "ConJS on the wasm VM prints otherwise than Node.js:"; diff "$tmp/node.out" "$tmp/wasm.out"; }
if [ "$m" = "$n" ]; then echo "== $name $n"; else echo "== $name $n (the slow column, marked *: $m)"; fi
echo "time: $what, the least of 3 runs"
printf '%-16s %s s\n' "Node.js" "$(best node "$tmp/fast.js")"
printf '%-16s %s s\n' "wasm VM *" "$(best "$conjs" "$tmp/slow.con.js")"

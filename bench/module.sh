#!/bin/bash
# bench/module.sh NAME/NAME.se OUT: make the ConMa module of a benchmark. The paths in it start with "conma/",
# as bin/consh writes them, so that the module has the same text wherever the repository is, and the
# transpiled code built from it (build.sh) is used. The modules are of fast ConMa (include/fast before include).
set -o pipefail
root="$(cd "$(dirname "$0")/.."; pwd)"
"$root/bin/includer" --include "$root/include/fast" --include "$root/include" "$1" | sed -e 's/"\/.*\/conma/"conma/' | "$root/bin/resolver" > "$2"

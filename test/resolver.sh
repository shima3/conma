#!/bin/bash
dir="$(cd ${0%/*}/..; pwd)"
bin="${dir}/bin"
base="${1%.se}"
ast="/tmp/${base}.ast.se"
# echo "$dir" "$bin" "$ast"
$bin/includer --include "$dir/include" "$1" | sed -e 's/"\/.*\/conma/"conma/' > "$ast"
$bin/resolver < "$ast" > "/tmp/${base}.module.se"

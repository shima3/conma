#!/bin/bash
dir="$(cd ${0%/*}; pwd)"
$dir/resolver.sh test.se
script -q /dev/null python3 $dir/vproc_run.py test.module.se $*

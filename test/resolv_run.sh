#!/bin/bash
dir="$(cd ${0%/*}; pwd)"
$dir/resolver.sh test.se
python3 $dir/vproc_run.py test.module.se $*

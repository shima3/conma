#!/bin/bash
dir="$(cd ${0%/*}; pwd)"
if ! $dir/resolver.sh test.se
then exit 1
fi
script -q /dev/null python3 $dir/vproc_run.py /tmp/test.module.se $*

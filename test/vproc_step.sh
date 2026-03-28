#!/bin/bash
if [ "$#" -lt 1 ]
then
    echo "Usage: ${0##*/} case?.vproc.se"
    exit 0
fi
dir="${0%/*}"
in="$1"
base="${1##*/}"
log="${base%.*}.log"
step=0
(
    echo "Step $step"
    cat "$in" | tee /tmp/in.vproc.se
    while true
    do
        step="$((step+1))"
        echo "Step $step"
        if ! python3 "$dir/vproc_step.py" "test.module.se" < /tmp/in.vproc.se > /tmp/out.vproc.se
        then exit 1
        fi
        cat /tmp/out.vproc.se
        mv /tmp/out.vproc.se /tmp/in.vproc.se
    done
) 2>&1 | tee /tmp/$log
exit 0

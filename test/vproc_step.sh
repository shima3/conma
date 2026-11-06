#!/bin/bash
# caseN.vproc.se の VProc を、test.module.se（カレントディレクトリ）で 1 ステップずつ実行し、
# 各ステップの VProc を標準出力に書く。VProc が終わるか、エラーになるまで続ける
if [ "$#" -lt 1 ]
then
    echo "Usage: ${0##*/} case?.vproc.se"
    exit 0
fi
dir="${0%/*}"
in="$1"
work="$(mktemp -d /tmp/vproc_step.XXXXXX)" || exit 1
trap 'rm -rf "$work"' EXIT
step=0
(
    echo "Step $step"
    cat "$in" | tee "$work/in.vproc.se"
    while true
    do
        step="$((step+1))"
        echo "Step $step"
        if ! python3 "$dir/vproc_step.py" "test.module.se" < "$work/in.vproc.se" > "$work/out.vproc.se"
        then exit 1
        fi
        cat "$work/out.vproc.se"
        mv "$work/out.vproc.se" "$work/in.vproc.se"
    done
) 2>&1
exit 0

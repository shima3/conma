#!/bin/bash
cd ${0%/*}
if ! ../resolver.sh test.se
then exit 1
fi
PYTHONUNBUFFERED=1 python3 ../vproc_run.py /tmp/test.module.se case1 > /tmp/case1.log 2>&1
diff case1.log /tmp/case1.log

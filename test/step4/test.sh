#!/bin/bash
cd ${0%/*}
if ! ../resolver.sh test.se
then exit 1
fi
python3 ../vproc_run.py /tmp/test.module.se case1 > /tmp/case1.log 2>&1
diff case1.log /tmp/case1.log
python3 ../vproc_run.py /tmp/test.module.se case2 > /tmp/case2.log 2>&1
diff case2.log /tmp/case2.log
python3 ../vproc_run.py /tmp/test.module.se case3 > /tmp/case3.log 2>&1
diff case3.log /tmp/case3.log

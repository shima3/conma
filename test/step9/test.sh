#!/bin/bash
cd ${0%/*}
../resolv_run.sh case1 > /tmp/case1.log 2>&1
diff case1.log /tmp/case1.log

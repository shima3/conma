#!/bin/bash
cd ${0%/*}
../vproc_step.sh case1.vproc.se > /dev/null
diff case1.vproc.log /tmp/case1.vproc.log
../vproc_step.sh case2.vproc.se > /dev/null
diff case2.vproc.log /tmp/case2.vproc.log

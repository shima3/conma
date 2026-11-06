#!/bin/bash
. "${0%/*}/../common.sh"
../vproc_step.sh case1.vproc.se > $tmp/case1.vproc.log
diff case1.vproc.log $tmp/case1.vproc.log
../vproc_step.sh case2.vproc.se > $tmp/case2.vproc.log
diff case2.vproc.log $tmp/case2.vproc.log
